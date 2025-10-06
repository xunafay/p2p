use libp2p::{
    dcutr, gossipsub, identify,
    kad::{self, store::MemoryStore},
    ping, relay,
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub relay_client: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub ping: ping::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}
