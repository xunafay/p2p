use std::sync::Arc;

use automerge::{ReadDoc, transaction::Transactable};
use futures::StreamExt;
use libp2p::{
    Multiaddr, Swarm, autonat, identify,
    kad::{self, QueryResult},
    multiaddr::Protocol,
    relay,
    swarm::SwarmEvent,
};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use tracing::{debug, info, warn};

use crate::behaviour::{Behaviour, BehaviourEvent};

pub enum SwarmCommand {
    Dial(Multiaddr),
    DialPeerId(libp2p::PeerId),
    BeginProviderRole(kad::RecordKey),
    StopProviderRole(kad::RecordKey),
    FindProviders(kad::RecordKey),
    ListConnections,
    PutTestValue(String, String),
    GetTestValue(String),
}

pub struct SwarmManager {
    swarm: Swarm<Behaviour>,
    event_tx: broadcast::Sender<Arc<SwarmEvent<BehaviourEvent>>>,
    command_rx: mpsc::Receiver<SwarmCommand>,
    relay_peer_id: libp2p::PeerId,
    relay_address: Multiaddr,
    sent_identify: bool,
    received_identify: bool,
}

impl SwarmManager {
    pub fn new(
        swarm: Swarm<Behaviour>,
        event_tx: broadcast::Sender<Arc<SwarmEvent<BehaviourEvent>>>,
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
                    self.handle_swarm_event(&event);
                    let _ = self.event_tx.send(Arc::new(event));
                }
                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        match command {
                            SwarmCommand::Dial(addr) => {
                                debug!("Dialing {}", addr);
                                match self.swarm.dial(addr.clone()) {
                                    Ok(()) => {
                                        debug!("Dialed {}", addr);
                                    }
                                    Err(err) => {
                                        debug!("Failed to dial {}: {:?}", addr, err);
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
                                        warn!("Failed to start providing for key: {:?}", err);
                                    }
                                }
                            }
                            SwarmCommand::StopProviderRole(key) => {
                                debug!("Stopping to provide for key {:?}", key);
                                self.swarm.behaviour_mut().kademlia.stop_providing(&key);
                                debug!("Stopped providing for key");
                            }
                            SwarmCommand::FindProviders(key) => {
                                debug!("Finding providers for key {:?}", key);
                                let query_id = self.swarm.behaviour_mut().kademlia.get_providers(key);
                                debug!("Started get_providers query with id {:?}", query_id);
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
                                debug!("Dialing peer id {}", peer_id);
                                match self.swarm.dial(peer_id) {
                                    Ok(()) => {
                                        debug!("Dialed peer {peer_id} successfully");
                                    }
                                    Err(err) => {
                                        debug!("Failed to dial peer id {}: {:?}", peer_id, err);
                                    }
                                }
                            },
                            SwarmCommand::PutTestValue(key, value) => {
                                tracing::info!("Putting test value {} at {}", value, key);
                                self.swarm.behaviour_mut().automerge.modify_document("test", |doc| {
                                    doc.put(automerge::ROOT, key, value).unwrap();
                                });
                            },
                            SwarmCommand::GetTestValue(key) => {
                                if let Some(doc) = self.swarm.behaviour_mut().automerge.get_document("test") {
                                    if let Ok(Some(value)) = doc.get(automerge::ROOT, &key) {
                                        tracing::info!("Got value for key {}: {:?}", key, value.0);
                                    } else {
                                        tracing::info!("Key {} not found in document", key);
                                    }
                                } else {
                                    tracing::info!("Document 'test' not found");
                                }
                            },
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

    fn handle_swarm_event(&mut self, event: &SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr {
                address,
                listener_id,
            } => {
                info!("Listening on {} (listener_id={})", address, listener_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    tracing::debug!("Failed to dial {peer_id}: {error:?}");
                } else {
                    tracing::debug!("Failed to dial unknown peer: {error:?}");
                }
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                cause,
                ..
            } => {
                if endpoint.is_relayed() {
                    tracing::debug!("Relay circuit closed from {peer_id} because {cause:?}");
                } else {
                    tracing::debug!("Connection closed from {peer_id} because {cause:?}");
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                debug!("Connected to {peer_id}, endpoint: {endpoint:?}");

                // bootstrap kademlia once connected to the relay
                // happens automatically?
                if &self.relay_peer_id == peer_id {
                    debug!("Connected to relay, starting kademlia bootstrap");
                    match self.swarm.behaviour_mut().kademlia.bootstrap() {
                        Ok(result) => {
                            debug!("Started kademlia bootstrap: {result:?}");
                        }
                        Err(err) => {
                            warn!("Failed to start kademlia bootstrap: {err:?}");
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
                peer_id,
                connection_id,
            })) => {
                if &self.relay_peer_id == peer_id {
                    tracing::debug!(
                        "Sent identify to relay {peer_id} via {connection_id}, should learn our public address soon"
                    );
                } else {
                    tracing::debug!("Sent identify to {peer_id} via {connection_id}");
                }
                self.sent_identify = true;
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
                result,
                tested_addr,
                server,
                ..
            })) => {
                let success = result.is_ok();
                tracing::debug!(%tested_addr, %server, success, "AutoNAT test completed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info: identify::Info { .. },
                peer_id,
                ..
            })) => {
                self.received_identify = true;
                // TODO only add observed addr if autonat says it's a public addr?

                if peer_id == &self.relay_peer_id && self.sent_identify {
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
                            for peer in &result.peers {
                                let peer_id = peer.peer_id;
                                debug!("Found peer {peer_id} via kademlia");

                                // TODO dial peer to punch hole
                            }
                        }
                        Err(err) => {
                            warn!("Failed to get closest peers: {err:?}");
                        }
                    },
                    QueryResult::Bootstrap(result) => match result {
                        Ok(result) => {
                            let peer = result.peer;
                            let num_remaining = result.num_remaining;
                            tracing::debug!(
                                "Kademlia bootstrap with {peer} completed, {num_remaining} queries remaining"
                            );
                        }
                        Err(err) => {
                            tracing::debug!("Kademlia bootstrap failed: {err:?}");
                        }
                    },
                    _ => {
                        tracing::debug!("Other kademlia query result: {result:?}");
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
                tracing::debug!(
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
                debug!("Relay circuit established via {relay_peer_id}, limit: {ttl}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                relay::client::Event::InboundCircuitEstablished { src_peer_id, limit },
            )) => {
                let limit = limit.unwrap();
                let ttl = limit.duration().unwrap().as_secs();
                debug!("Inbound relay circuit established from {src_peer_id}, limit: {ttl}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Dcutr(libp2p::dcutr::Event {
                remote_peer_id,
                result,
            })) => match result {
                Ok(_) => {
                    info!("DCUtR with {remote_peer_id} succeeded");
                }
                Err(err) => {
                    warn!("DCUtR with {remote_peer_id} failed: {err:?}");
                }
            },
            _ => {}
        }
    }
}
