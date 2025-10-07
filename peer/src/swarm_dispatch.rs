use futures::StreamExt;
use libp2p::{
    Multiaddr, Swarm, autonat, identify,
    kad::{self, QueryResult},
    multiaddr::Protocol,
    relay,
    swarm::SwarmEvent,
};
use tokio::{select, sync::mpsc};
use tracing::{info, warn};

use crate::behaviour::{Behaviour, BehaviourEvent};

pub enum SwarmCommand {
    Dial(Multiaddr),
    DialPeerId(libp2p::PeerId),
    BeginProviderRole(kad::RecordKey),
    StopProviderRole(kad::RecordKey),
    FindProviders(kad::RecordKey),
    ListConnections,
}

pub struct SwarmManager {
    swarm: Swarm<Behaviour>,
    event_tx: mpsc::Sender<SwarmEvent<BehaviourEvent>>,
    command_rx: mpsc::Receiver<SwarmCommand>,
    relay_peer_id: libp2p::PeerId,
    relay_address: Multiaddr,
    sent_identify: bool,
    received_identify: bool,
}

impl SwarmManager {
    pub fn new(
        swarm: Swarm<Behaviour>,
        event_tx: mpsc::Sender<SwarmEvent<BehaviourEvent>>,
        command_rx: mpsc::Receiver<SwarmCommand>,
        relay_peer_id: libp2p::PeerId,
        relay_address: Multiaddr,
    ) -> Self {
        SwarmManager {
            swarm,
            event_tx,
            command_rx,
            relay_peer_id,
            sent_identify: false,
            received_identify: false,
            relay_address,
        }
    }

    pub async fn run(mut self) {
        info!("SwarmManager started");
        loop {
            select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event);
                }
                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        match command {
                            SwarmCommand::Dial(addr) => {
                                info!("Dialing {}", addr);
                                match self.swarm.dial(addr.clone()) {
                                    Ok(()) => {
                                        info!("Dialed {}", addr);
                                    }
                                    Err(err) => {
                                        info!("Failed to dial {}: {:?}", addr, err);
                                    }
                                }
                            }
                            SwarmCommand::BeginProviderRole(key) => {
                                info!("Starting to provide for key {:?}", key);
                                match self.swarm.behaviour_mut().kademlia.start_providing(key) {
                                    Ok(_) => {
                                        info!("Started providing for key");
                                    }
                                    Err(err) => {
                                        info!("Failed to start providing for key: {:?}", err);
                                    }
                                }
                            }
                            SwarmCommand::StopProviderRole(key) => {
                                info!("Stopping to provide for key {:?}", key);
                                self.swarm.behaviour_mut().kademlia.stop_providing(&key);
                                info!("Stopped providing for key");
                            }
                            SwarmCommand::FindProviders(key) => {
                                info!("Finding providers for key {:?}", key);
                                let query_id = self.swarm.behaviour_mut().kademlia.get_providers(key);
                                info!("Started get_providers query with id {:?}", query_id);
                            }
                            SwarmCommand::ListConnections => {
                                let connections = self.swarm.connected_peers().collect::<Vec<_>>();
                                if connections.is_empty() {
                                    info!("No active connections");
                                } else {
                                    info!("Active connections:");
                                    for conn in connections {
                                        info!(" - {:?}", conn);
                                    }
                                }
                            }
                            SwarmCommand::DialPeerId(peer_id) => {
                                info!("Dialing peer id {}", peer_id);
                                match self.swarm.dial(peer_id) {
                                    Ok(()) => {
                                        info!("Dialed peer {peer_id} successfully");
                                    }
                                    Err(err) => {
                                        info!("Failed to dial peer id {}: {:?}", peer_id, err);
                                    }
                                }
                            }
                        }
                    } else {
                        // command channel closed
                        info!("Command channel closed, exiting SwarmManager");
                        break;
                    }
                }
            }
        }
    }
    fn handle_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr {
                address,
                listener_id,
            } => {
                info!("Listening on {} (listener_id={})", address, listener_id);
            }
            SwarmEvent::OutgoingConnectionError {
                peer_id,
                error,
                connection_id,
                ..
            } => {
                if let Some(peer_id) = peer_id {
                    tracing::info!("Failed to dial {peer_id}: {error:?}");
                } else {
                    tracing::info!("Failed to dial unknown peer: {error:?}");
                }
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
            // SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event {
            //     result,
            //     peer,
            //     connection,
            // })) => match result {
            //     Ok(rtt) => {
            //         info!("Ping to {peer} via {connection} is {rtt:?}");
            //     }
            //     Err(err) => {
            //         info!("Ping to {peer} via {connection} failed: {err:?}");
            //     }
            // },
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("Connected to {peer_id}, endpoint: {endpoint:?}");

                // bootstrap kademlia once connected to the relay
                // happens automatically?
                if self.relay_peer_id == peer_id {
                    info!("Connected to relay, starting kademlia bootstrap");
                    match self.swarm.behaviour_mut().kademlia.bootstrap() {
                        Ok(result) => {
                            info!("Started kademlia bootstrap: {result:?}");
                        }
                        Err(err) => {
                            info!("Failed to start kademlia bootstrap: {err:?}");
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
                peer_id,
                connection_id,
            })) => {
                if self.relay_peer_id == peer_id {
                    tracing::info!(
                        "Sent identify to relay {peer_id} via {connection_id}, should learn our public address soon"
                    );
                } else {
                    tracing::info!("Sent identify to {peer_id} via {connection_id}");
                }
                self.sent_identify = true;
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::Event::StatusChanged {
                old,
                new,
            })) => {
                tracing::info!("Autonat status changed from {old:?} to {new:?}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info: identify::Info { observed_addr, .. },
                peer_id,
                ..
            })) => {
                self.received_identify = true;
                // TODO only add observed addr if autonat says it's a public addr?

                if peer_id == self.relay_peer_id && self.sent_identify {
                    tracing::info!(address=%observed_addr, "Idfk anymore what i'm doing here ngl");

                    let circuit_addr = self
                        .relay_address
                        .clone()
                        .with(Protocol::P2p(self.relay_peer_id))
                        .with(Protocol::P2pCircuit);

                    self.swarm.listen_on(circuit_addr.clone()).unwrap();
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed { result, .. },
            )) => {
                match result {
                    QueryResult::GetClosestPeers(result) => match result {
                        Ok(result) => {
                            for peer in result.peers {
                                let peer_id = peer.peer_id;
                                info!("Found peer {peer_id} via kademlia");

                                // TODO dial peer to punch hole
                            }
                        }
                        Err(err) => {
                            info!("Failed to get closest peers: {err:?}");
                        }
                    },
                    QueryResult::Bootstrap(result) => match result {
                        Ok(result) => {
                            let peer = result.peer;
                            let num_remaining = result.num_remaining;
                            tracing::info!(
                                "Kademlia bootstrap with {peer} completed, {num_remaining} queries remaining"
                            );
                        }
                        Err(err) => {
                            tracing::info!("Kademlia bootstrap failed: {err:?}");
                        }
                    },
                    _ => {
                        tracing::info!("Other kademlia query result: {result:?}");
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted {
                    relay_peer_id,
                    renewal,
                    limit,
                },
            )) => {
                let limit = limit.unwrap();
                let ttl = limit.duration().unwrap().as_secs();
                tracing::info!(
                    "Relay reservation accepted from {relay_peer_id}, renewal: {renewal:?}, limit: {ttl}"
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                relay::client::Event::OutboundCircuitEstablished {
                    relay_peer_id,
                    limit,
                },
            )) => {
                let limit = limit.unwrap();
                let ttl = limit.duration().unwrap().as_secs();
                info!("Relay circuit established via {relay_peer_id}, limit: {ttl}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                relay::client::Event::InboundCircuitEstablished { src_peer_id, limit },
            )) => {
                let limit = limit.unwrap();
                let ttl = limit.duration().unwrap().as_secs();
                info!("Inbound relay circuit established from {src_peer_id}, limit: {ttl}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Dcutr(libp2p::dcutr::Event {
                remote_peer_id,
                result,
            })) => match result {
                Ok(_) => {
                    info!("DCUtR with {remote_peer_id} succeeded");
                }
                Err(err) => {
                    info!("DCUtR with {remote_peer_id} failed: {err:?}");
                }
            },
            event => {
                // println!("unmanaged event: {event:?}");
            }
        }
    }
}
