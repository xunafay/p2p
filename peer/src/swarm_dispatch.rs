use futures::StreamExt;
use libp2p::{
    Multiaddr, Swarm, identify,
    kad::{self, QueryResult},
    swarm::SwarmEvent,
};
use tokio::{select, sync::mpsc};
use tracing::info;

use crate::behaviour::{Behaviour, BehaviourEvent};

pub enum SwarmCommand {
    Dial(Multiaddr),
    BeginProviderRole(kad::RecordKey),
    StopProviderRole(kad::RecordKey),
    FindProviders(kad::RecordKey),
}

pub struct SwarmManager {
    swarm: Swarm<Behaviour>,
    event_tx: mpsc::Sender<SwarmEvent<BehaviourEvent>>,
    command_rx: mpsc::Receiver<SwarmCommand>,
    relay_peer_id: libp2p::PeerId,
}

impl SwarmManager {
    pub fn new(
        swarm: Swarm<Behaviour>,
        event_tx: mpsc::Sender<SwarmEvent<BehaviourEvent>>,
        command_rx: mpsc::Receiver<SwarmCommand>,
        relay_peer_id: libp2p::PeerId,
    ) -> Self {
        SwarmManager {
            swarm,
            event_tx,
            command_rx,
            relay_peer_id,
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
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent { .. })) => {
                tracing::info!("Told relay its public address");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info: identify::Info { observed_addr, .. },
                peer_id,
                ..
            })) => {
                self.swarm.add_external_address(observed_addr.clone());
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, observed_addr.clone());
                tracing::info!(address=%observed_addr, "Relay told us our observed address, added to external addresses and kademlia");
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
