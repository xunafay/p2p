use std::task::Poll;

use libp2p::{
    PeerId, Stream, StreamProtocol,
    autonat::v2::client::Event,
    core::upgrade::{DeniedUpgrade, ReadyUpgrade},
    swarm::{ConnectionHandler, ConnectionHandlerEvent, SubstreamProtocol},
};
use tracing::warn;

use crate::protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub enum Command {
    StartSync {
        document_id: String,
        peer: PeerId,
    },
    SendChanges {
        document_id: String,
        changes: Vec<u8>,
        peer: PeerId,
    },
    BroadcastChanges {
        document_id: String,
        changes: Vec<u8>,
    },
    RequestSync {
        document_id: String,
        peer: PeerId,
    },
}

/// Event from behaviour to the connection handler
#[derive(Debug)]
pub enum InEvent {
    DocumentChanged { document_id: String },
}

pub struct Handler {
    pending_events: Vec<Event>,
}

impl Handler {
    pub fn new() -> Self {
        Handler {
            pending_events: Vec::new(),
        }
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = InEvent;
    type ToBehaviour = Event;
    type InboundProtocol = ReadyUpgrade<StreamProtocol>;
    type OutboundProtocol = ReadyUpgrade<StreamProtocol>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
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
        if let Some(event) = self.pending_events.pop() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        warn!("Received behaviour event: {:?}", event);
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
        tracing::debug!("Connection handler event: {:?}", event);
    }
}

enum OutboundState {
    PedingStream,
    Ready(Stream),
}
