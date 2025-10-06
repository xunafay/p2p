use std::{error::Error, str::FromStr, time::Duration};

use clap::Parser;
use futures::stream::StreamExt;
use libp2p::{
    PeerId, autonat,
    core::multiaddr::Multiaddr,
    dcutr, gossipsub, identify, identity,
    kad::{self, QueryResult, store::MemoryStore},
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use sha2::{Digest, Sha256};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    swarm_dispatch::SwarmManager,
};

pub mod behaviour;
pub mod database_manager;
pub mod swarm_dispatch;

#[derive(Debug, Parser)]
#[command(name = "libp2p DCUtR client")]
struct Opts {
    /// Fixed value to generate deterministic peer id.
    #[arg(long)]
    secret_key_seed: Option<u8>,

    /// The listening address
    #[arg(long)]
    relay_address: Multiaddr,

    /// Peer ID of the remote peer to hole punch to.
    #[arg(long)]
    relay_peer_id: PeerId,

    /// Pre-shared key for Noise protocol
    ///
    /// Example: "mysecretkey"
    #[arg(long)]
    key: String,
}

/// Hashes a string to a [u8; 32] key using SHA-256.
fn string_to_32_bytes(s: &str) -> [u8; 32] {
    let hash = Sha256::digest(s.as_bytes());
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash[..]);
    arr
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("info".parse()?)
                .from_env()
                .unwrap(),
        )
        .try_init();

    let opts = Opts::parse();

    let keypair = if let Some(seed) = opts.secret_key_seed {
        generate_ed25519_from_seed(seed)
    } else {
        generate_ed25519()
    };

    let mut kademlia = libp2p::kad::Behaviour::new(
        keypair.public().to_peer_id(),
        MemoryStore::new(keypair.public().to_peer_id()),
    );
    kademlia.set_mode(Some(kad::Mode::Client));
    kademlia.add_address(&opts.relay_peer_id, opts.relay_address.clone());

    let noise_config_with_prologue =
        |keypair: &identity::Keypair| -> Result<noise::Config, std::io::Error> {
            let mut config = noise::Config::new(keypair).expect("Noise key generation failed");
            config = config.with_prologue(string_to_32_bytes(&opts.key).to_vec());
            Ok(config)
        };

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise_config_with_prologue,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|keypair, relay_behaviour| Behaviour {
            relay_client: relay_behaviour,
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
            identify: identify::Behaviour::new(identify::Config::new(
                "/TODO/0.0.1".to_string(),
                keypair.public(),
            )),
            autonat: autonat::Behaviour::new(
                keypair.clone().public().to_peer_id(),
                autonat::Config {
                    retry_interval: Duration::from_secs(10),
                    refresh_interval: Duration::from_secs(30),
                    boot_delay: Duration::from_secs(5),
                    throttle_server_period: Duration::ZERO,
                    only_global_ips: false,
                    ..Default::default()
                },
            ),
            dcutr: dcutr::Behaviour::new(keypair.public().to_peer_id()),
            gossipsub: gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                gossipsub::Config::default(),
            )
            .unwrap(),
            kademlia,
        })?
        .with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
        .unwrap();
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    // Connect to the relay server. Not for the reservation or relayed connection, but to (a) learn
    // our local public address and (b) enable a freshly started relay to learn its public address.
    swarm
        .dial(
            opts.relay_address
                .clone()
                .with_p2p(opts.relay_peer_id)
                .unwrap(),
        )
        .unwrap();

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let ctrl_c_signal = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c_signal);

    let mut is_db_provider = false;
    let db_key = kad::RecordKey::new(&"db".as_bytes().to_vec());
    // swarm
    //     .behaviour_mut()
    //     .kademlia
    //     .start_providing(db_key.clone())
    //     .expect("failed to start providing as kademlia relay");

    let (swarm_event_tx, swarm_event_rx) =
        tokio::sync::mpsc::channel::<SwarmEvent<BehaviourEvent>>(32);
    let (swarm_command_tx, swarm_command_rx) =
        tokio::sync::mpsc::channel::<swarm_dispatch::SwarmCommand>(32);

    let swarm_manager = SwarmManager::new(
        swarm,
        swarm_event_tx,
        swarm_command_rx,
        opts.relay_peer_id,
        opts.relay_address.clone(),
    );

    tokio::spawn(async move { swarm_manager.run().await });

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                let line = line.trim();
                if line == "exit" || line == "quit" || line == "q" {
                    info!("exiting...");
                    break;
                } else if line == "promote db" {
                    if !is_db_provider {
                        info!("promoting to db provider");
                        swarm_command_tx.send(swarm_dispatch::SwarmCommand::BeginProviderRole(db_key.clone())).await.unwrap();
                        is_db_provider = true;
                    } else {
                        info!("already a db provider");
                    }
                } else if line == "demote db" {
                    if is_db_provider {
                        info!("demoting from db provider");
                        swarm_command_tx.send(swarm_dispatch::SwarmCommand::StopProviderRole(db_key.clone())).await.unwrap();
                        is_db_provider = false;
                    }
                    else {
                        info!("not a db provider");
                    }
                } else if line.starts_with("get providers ") {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let key_str = parts[2];
                        let key = kad::RecordKey::new(&key_str.as_bytes().to_vec());
                        info!("looking for providers of key: {}", key_str);
                        swarm_command_tx.send(swarm_dispatch::SwarmCommand::FindProviders(key)).await.unwrap();
                    } else {
                        warn!("usage: get providers <key>");
                    }
                } else if line.starts_with("dial") {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let peer_id = parts[1];
                                let addr = opts.relay_address
                                    .clone()
                                    .with(Protocol::P2p(opts.relay_peer_id))
                                    .with(Protocol::P2pCircuit)
                                    .with(Protocol::P2p(PeerId::from_str(peer_id).unwrap()));
                                info!("dialing {}", addr);
                                swarm_command_tx.send(swarm_dispatch::SwarmCommand::Dial(addr)).await.unwrap();
                    } else {
                        warn!("usage: dial <multiaddr>");
                    }
                } else {
                    warn!("unknown command: {}", line);
                }
            },
            _ = &mut ctrl_c_signal => {
                info!("received Ctrl-C, shutting down.");

                break;
            },
        }
    }

    Ok(())
}

fn generate_ed25519() -> identity::Keypair {
    identity::Keypair::generate_ed25519()
}

fn generate_ed25519_from_seed(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
}
