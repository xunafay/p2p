use std::{error::Error, str::FromStr, sync::Arc, time::Duration};

use clap::Parser;
use futures::stream::StreamExt;
use libp2p::{
    PeerId, autonat,
    core::multiaddr::Multiaddr,
    dcutr, gossipsub, identify,
    identity::{self, ed25519},
    kad::{self, QueryResult, store::MemoryStore},
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    database_manager::DatabaseManager,
    local_config::AppConfig,
    swarm_dispatch::SwarmManager,
};

pub mod behaviour;
pub mod database_manager;
pub mod local_config;
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

fn get_config_or_default() -> Result<local_config::AppConfig, Box<dyn Error>> {
    if let Ok(config) = local_config::AppConfig::load() {
        config.validate()?;
        return Ok(config);
    };

    AppConfig::default().save()?;
    Err(format!("No valid config found. A default config has been created at {}. Please edit it and restart the application.", AppConfig::default_config_location()).into())
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

    let peer_config = get_config_or_default().unwrap_or_else(|e| {
        println!("{}", e);
        std::process::exit(1);
    });

    let keypair = peer_config.load_keypair().expect("Failed to load keypair");
    let mut kademlia = libp2p::kad::Behaviour::new(
        keypair.public().to_peer_id(),
        MemoryStore::new(keypair.public().to_peer_id()),
    );
    kademlia.set_mode(Some(kad::Mode::Client));
    kademlia.add_address(
        &peer_config.relay.peer_id,
        peer_config.relay.address.clone(),
    );

    let noise_config_with_prologue =
        |keypair: &identity::Keypair| -> Result<noise::Config, std::io::Error> {
            let mut config = noise::Config::new(keypair).expect("Noise key generation failed");
            config = config
                .with_prologue(string_to_32_bytes(&peer_config.identity.pre_shared_key).to_vec());
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
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(30))),
            identify: identify::Behaviour::new(
                identify::Config::new("ipfs/1.0.0".to_owned(), keypair.public())
                    .with_hide_listen_addrs(false)
                    .with_push_listen_addr_updates(true),
            ),
            autonat: autonat::v2::client::Behaviour::new(
                OsRng,
                autonat::v2::client::Config::default(),
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
            peer_config
                .relay
                .address
                .clone()
                .with_p2p(peer_config.relay.peer_id)
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
        tokio::sync::broadcast::channel::<Arc<SwarmEvent<BehaviourEvent>>>(32);
    let (swarm_command_tx, swarm_command_rx) =
        tokio::sync::mpsc::channel::<swarm_dispatch::SwarmCommand>(32);
    let (db_event_tx, db_event_rx) =
        tokio::sync::mpsc::channel::<database_manager::DatabaseEvent>(32);
    let (db_command_tx, db_command_rx) =
        tokio::sync::mpsc::channel::<database_manager::DatabaseCommand>(32);

    let swarm_manager = SwarmManager::new(
        swarm,
        swarm_event_tx,
        swarm_command_rx,
        peer_config.relay.peer_id,
        peer_config.relay.address.clone(),
    );

    let database_manager = DatabaseManager::new(
        db_event_tx,
        db_command_rx,
        swarm_event_rx,
        swarm_command_tx.clone(),
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
                                let addr = peer_config.relay.address
                                    .clone()
                                    .with(Protocol::P2p(peer_config.relay.peer_id))
                                    .with(Protocol::P2pCircuit)
                                    .with(Protocol::P2p(PeerId::from_str(peer_id).unwrap()));
                                info!("dialing {}", addr);
                                swarm_command_tx.send(swarm_dispatch::SwarmCommand::Dial(addr)).await.unwrap();
                    } else {
                        warn!("usage: dial <multiaddr>");
                    }
                } else if line.starts_with("dial_id") {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let peer_id = parts[1];
                        let peer_id = PeerId::from_str(peer_id).unwrap();
                        info!("dialing peer id {}", peer_id);
                        swarm_command_tx.send(swarm_dispatch::SwarmCommand::DialPeerId(peer_id)).await.unwrap();
                    } else {
                        warn!("usage: dial_id <peer_id>");
                    }
                } else if line.starts_with("connections") {
                    swarm_command_tx.send(swarm_dispatch::SwarmCommand::ListConnections).await.unwrap();
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
