use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
};

use either::Either::{self, Left};
use libp2p::{
    PeerId,
    swarm::{NetworkBehaviour, ToSwarm, dummy},
};

use crate::handler::{Command, Handler};

#[derive(Debug)]
pub enum Event {
    DocumentSynced {
        peer: PeerId,
        document_id: String,
    },
    DocumentChanged {
        peer: PeerId,
        document_id: String,
    },
    SyncRequested {
        peer: PeerId,
        document_id: String,
    },
    SyncError {
        peer: PeerId,
        document_id: String,
        error: String,
    },
}

pub struct Behaviour {
    /// Events to be sent to the swarm
    queued_events: VecDeque<ToSwarm<Event, Either<Command, Infallible>>>,
    /// (PeerId, DocumentId)
    active_syncs: Vec<(PeerId, String)>,
    /// Pending commands to send to connection handlers
    pending_commands: HashMap<(PeerId, String), VecDeque<Command>>,
}

impl Behaviour {
    pub fn new() -> Self {
        Behaviour {
            queued_events: VecDeque::new(),
            active_syncs: Vec::new(),
            pending_commands: HashMap::new(),
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = Either<Handler, dummy::ConnectionHandler>;

    type ToSwarm = Event;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: libp2p::PeerId,
        local_addr: &libp2p::Multiaddr,
        remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        tracing::warn!("Established inbound connection: {:?}", peer);
        Ok(Either::Left(crate::handler::Handler::new()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: libp2p::PeerId,
        addr: &libp2p::Multiaddr,
        role_override: libp2p::core::Endpoint,
        port_use: libp2p::core::transport::PortUse,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        tracing::warn!("Established outbound connection: {:?}", peer);
        Ok(Either::Left(crate::handler::Handler::new()))
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        tracing::warn!("Connection handler event: {:?}", event);
        match event {
            Left(handler_event) => match handler_event {
                _ => {
                    tracing::warn!("Unhandled handler event: {:?}", handler_event);
                }
            },
            _ => {} // dummy handler, do nothing
        }
    }

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>> {
        if let Some(event) = self.queued_events.pop_front() {
            return std::task::Poll::Ready(event);
        } else if self.queued_events.capacity() > 100 {
            self.queued_events.shrink_to_fit();
        }

        std::task::Poll::Pending
    }
}
