use std::{
    error::Error,
    net::{Ipv4Addr, Ipv6Addr},
    num::NonZeroU32,
    time::Duration,
};

use clap::Parser;
use futures::StreamExt;
use libp2p::{
    autonat,
    core::{Multiaddr, multiaddr::Protocol},
    identify, identity,
    kad::{self, store::MemoryStore},
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use tracing_subscriber::EnvFilter;

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

    let opts = Opt::parse();

    let local_key = if let Some(seed) = opts.secret_key_seed {
        generate_ed25519_from_seed(seed)
    } else {
        generate_ed25519()
    };

    let mut kademlia = libp2p::kad::Behaviour::new(
        local_key.public().to_peer_id(),
        MemoryStore::new(local_key.public().to_peer_id()),
    );
    kademlia.set_mode(Some(kad::Mode::Server));

    let noise_config_with_prologue =
        |keypair: &identity::Keypair| -> Result<noise::Config, std::io::Error> {
            let mut config = noise::Config::new(keypair).expect("Noise key generation failed");
            config = config.with_prologue(string_to_32_bytes(&opts.key).to_vec());
            Ok(config)
        };

    let mut relay_config = relay::Config::default()
        .reservation_rate_per_peer(NonZeroU32::new(60).unwrap(), Duration::from_secs(60 * 60))
        .reservation_rate_per_ip(NonZeroU32::new(1000).unwrap(), Duration::from_secs(60 * 60))
        .circuit_src_per_ip(NonZeroU32::new(1000).unwrap(), Duration::from_secs(60 * 60))
        .circuit_src_per_peer(NonZeroU32::new(500).unwrap(), Duration::from_secs(60 * 60));

    relay_config.max_circuit_bytes = 5 * 1024 * 1024 * 1024; // 5 gibibyte

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise_config_with_prologue,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| Behaviour {
            relay: relay::Behaviour::new(key.public().to_peer_id(), relay_config),
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(20))),
            identify: identify::Behaviour::new(
                identify::Config::new("ipfs/1.0.0".to_owned(), key.public())
                    .with_hide_listen_addrs(false)
                    .with_push_listen_addr_updates(true),
            ),
            kademlia,
            autonat: autonat::v2::server::Behaviour::new(OsRng),
        })?
        .with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on all interfaces
    let listen_addr_tcp = Multiaddr::empty()
        .with(match opts.use_ipv6 {
            Some(true) => Protocol::from(Ipv6Addr::UNSPECIFIED),
            _ => Protocol::from(Ipv4Addr::UNSPECIFIED),
        })
        .with(Protocol::Tcp(opts.port));
    swarm.listen_on(listen_addr_tcp.clone())?;

    let listen_addr_quic = Multiaddr::empty()
        .with(match opts.use_ipv6 {
            Some(true) => Protocol::from(Ipv6Addr::UNSPECIFIED),
            _ => Protocol::from(Ipv4Addr::UNSPECIFIED),
        })
        .with(Protocol::Udp(opts.port))
        .with(Protocol::QuicV1);
    swarm.listen_on(listen_addr_quic)?;

    swarm
        .behaviour_mut()
        .kademlia
        .start_providing(local_key.clone().public().to_peer_id().to_bytes().into())
        .expect("failed to start providing as kademlia relay");

    loop {
        match swarm.next().await.expect("Infinite Stream.") {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address:?}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::server::Event {
                result,
                tested_addr,
                client,
                ..
            })) => {
                let success = result.is_ok();
                tracing::info!(%tested_addr, %client, success, "AutoNAT test completed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info: identify::Info { observed_addr, .. },
                peer_id,
                ..
            })) => {
                swarm.add_external_address(observed_addr.clone());
                let addr = observed_addr
                    .clone()
                    .with(Protocol::P2p(local_key.public().to_peer_id()))
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(peer_id));

                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());

                tracing::info!("-> Stored address for {peer_id}: {addr:?}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::Event::ReservationReqAccepted { src_peer_id, .. },
            )) => {
                tracing::info!("Reservation request accepted from {src_peer_id}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                peer,
                ..
            })) => {
                tracing::info!(%peer, "Kademlia routing table updated for {peer}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
                connection_id,
                peer_id,
            })) => {
                tracing::info!(%connection_id, %peer_id, "Sent identify info");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
                ..
            })) => {
                tracing::info!("Circuit request accepted from {src_peer_id} <-> {dst_peer_id}");
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                cause,
            } => {
                if endpoint.is_relayed() {
                    tracing::info!("Relay circuit closed from {peer_id} because {cause:?}");
                } else {
                    tracing::info!("Connection closed from {peer_id} because {cause:?}");
                }
            }
            event => {
                // tracing::info!("{event:?}")
            }
        }
    }
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::Behaviour,
    identify: identify::Behaviour,
    kademlia: libp2p::kad::Behaviour<MemoryStore>,
    ping: ping::Behaviour,
    autonat: autonat::v2::server::Behaviour,
}

fn generate_ed25519() -> identity::Keypair {
    identity::Keypair::generate_ed25519()
}

fn generate_ed25519_from_seed(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
}

#[derive(Debug, Parser)]
#[command(name = "libp2p relay")]
struct Opt {
    /// Determine if the relay listen on ipv6 or ipv4 loopback address. the default is ipv4
    #[arg(long)]
    use_ipv6: Option<bool>,

    /// Fixed value to generate deterministic peer id
    #[arg(long)]
    secret_key_seed: Option<u8>,

    /// The port used to listen on all interfaces
    #[arg(long)]
    port: u16,

    /// Pre-shared key for Noise protocol
    ///
    /// Example: "mysecretkey"
    #[arg(long)]
    key: String,
}
