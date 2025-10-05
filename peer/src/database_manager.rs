use libp2p::Multiaddr;
use tokio::sync::mpsc;
use tracing::info;

pub enum DatabaseCommand {
    Bootstrap,
    DiscoverProviders,
    RequestUpgradeToProvider(Multiaddr),
}

pub enum DatabaseEvent {
    DiscoveredProviders(Vec<Multiaddr>),
    RequestUpgradeToProvider,
}

pub struct DatabaseManager {
    event_rx: mpsc::Receiver<DatabaseEvent>,
    command_tx: mpsc::Sender<DatabaseCommand>,
}

impl DatabaseManager {
    pub fn new(
        event_rx: mpsc::Receiver<DatabaseEvent>,
        command_tx: mpsc::Sender<DatabaseCommand>,
    ) -> Self {
        DatabaseManager {
            event_rx,
            command_tx,
        }
    }

    pub async fn run(mut self) {
        info!("DatabaseManager started");
    }
}
