use std::sync::Arc;

use libp2p::{Multiaddr, swarm::SwarmEvent};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use tracing::info;

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    swarm_dispatch::SwarmCommand,
};

pub enum DatabaseCommand {
    RequestUpgradeToProvider(Multiaddr),
}

pub enum DatabaseEvent {
    RequestUpgradeToProvider,
}

pub struct DatabaseManager {
    event_tx: mpsc::Sender<DatabaseEvent>,
    command_rx: mpsc::Receiver<DatabaseCommand>,
    swarm_command_tx: mpsc::Sender<SwarmCommand>,
    swarm_event_rx: broadcast::Receiver<Arc<SwarmEvent<BehaviourEvent>>>,
}

impl DatabaseManager {
    pub fn new(
        event_tx: mpsc::Sender<DatabaseEvent>,
        command_rx: mpsc::Receiver<DatabaseCommand>,
        swarm_event_rx: broadcast::Receiver<Arc<SwarmEvent<BehaviourEvent>>>,
        swarm_command_tx: mpsc::Sender<SwarmCommand>,
    ) -> Self {
        DatabaseManager {
            event_tx,
            command_rx,
            swarm_command_tx,
            swarm_event_rx,
        }
    }

    pub async fn run(mut self) {
        info!("DatabaseManager started");

        loop {
            select! {
                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        self.handle_command(command);
                    } else {
                        info!("DatabaseManager command channel closed, shutting down");
                        break;
                    }
                }

                event = self.swarm_event_rx.recv() => {
                    if let Ok(event) = event {
                        self.handle_swarm_event(event);
                    }
                }
            }
        }
    }

    pub fn handle_command(&mut self, command: DatabaseCommand) {
        match command {
            DatabaseCommand::RequestUpgradeToProvider(addr) => {
                info!("Requesting upgrade to provider at {}", addr);
            }
        }
    }

    pub fn handle_swarm_event(&mut self, event: Arc<SwarmEvent<BehaviourEvent>>) {}
}
