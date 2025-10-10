use std::{collections::VecDeque, convert::Infallible};

use either::Either;
use libp2p::swarm::{NetworkBehaviour, ToSwarm, dummy};

use crate::handler::{Command, Handler};

#[derive(Debug)]
pub enum Event {}

pub struct Behaviour {
    queued_events: VecDeque<ToSwarm<Event, Either<Command, Infallible>>>,
}

impl Behaviour {
    pub fn new() -> Self {
        Behaviour {
            queued_events: VecDeque::new(),
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
        todo!()
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: libp2p::PeerId,
        addr: &libp2p::Multiaddr,
        role_override: libp2p::core::Endpoint,
        port_use: libp2p::core::transport::PortUse,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        todo!()
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        todo!()
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        _event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        todo!()
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
