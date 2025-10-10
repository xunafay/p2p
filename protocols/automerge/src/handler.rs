use libp2p::{
    StreamProtocol,
    autonat::v2::client::Event,
    core::upgrade::{DeniedUpgrade, ReadyUpgrade},
    swarm::ConnectionHandler,
};

#[derive(Debug)]
pub enum Command {}

pub struct Handler {}
impl ConnectionHandler for Handler {
    type FromBehaviour = Command;

    type ToBehaviour = Event;

    type InboundProtocol = ReadyUpgrade<StreamProtocol>;

    type OutboundProtocol = DeniedUpgrade;

    type InboundOpenInfo = ();

    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
        >,
    > {
        todo!()
    }

    fn on_behaviour_event(&mut self, _event: Self::FromBehaviour) {
        todo!()
    }

    fn on_connection_event(
        &mut self,
        event: libp2p::swarm::handler::ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        todo!()
    }
}
