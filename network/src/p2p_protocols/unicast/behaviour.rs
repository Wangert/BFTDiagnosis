use std::{
    collections::{hash_map::DefaultHasher, HashSet, VecDeque},
    task::Poll,
    time::Duration,
};

use cuckoofilter::{CuckooError, CuckooFilter};
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint},
    swarm::{
        dial_opts::{self, DialOpts},
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler,
        OneShotHandlerConfig, SubstreamProtocol,
    },
    Multiaddr, PeerId,
};
use rand::Rng;

use super::protocol::{UnicastMessage, UnicastProtocol};

#[derive(Debug)]
pub enum UnicastEvent {
    Message(UnicastMessage),
}

#[derive(Debug)]
pub enum InnerMessage {
    Rx(UnicastMessage),
    Sent,
}

impl From<UnicastMessage> for InnerMessage {
    #[inline]
    fn from(msg: UnicastMessage) -> InnerMessage {
        InnerMessage::Rx(msg)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Sent
    }
}

pub struct Unicast {
    // unicast event list
    events: VecDeque<
        NetworkBehaviourAction<
            UnicastEvent,
            OneShotHandler<UnicastProtocol, UnicastMessage, InnerMessage>,
        >,
    >,
    // basic config
    config: UnicastConfig,
    // peers that can sent
    can_send_peers: Vec<PeerId>,
    // connected peer list
    connected_peers: HashSet<PeerId>,
    // same message filter
    received: CuckooFilter<DefaultHasher>,
}

impl Unicast {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self::from_config(UnicastConfig::new(local_peer_id))
    }

    pub fn from_config(config: UnicastConfig) -> Self {
        Unicast {
            events: VecDeque::new(),
            config,
            can_send_peers: Vec::new(),
            connected_peers: HashSet::new(),
            received: CuckooFilter::new(),
        }
    }

    #[inline]
    pub fn add_node_to_partial_view(&mut self, peer_id: &PeerId) {
        // if self.can_send_peers.insert(*peer_id) {
        //     let handler = self.new_handler();
        //     self.events.push_back(NetworkBehaviourAction::Dial {
        //         opts: DialOpts::peer_id(*peer_id)
        //             .condition(dial_opts::PeerCondition::Disconnected)
        //             .build(),
        //         handler,
        //     });
        // }

        self.can_send_peers.push(*peer_id);
        let handler = self.new_handler();
        self.events.push_back(NetworkBehaviourAction::Dial {
            opts: DialOpts::peer_id(*peer_id)
                .condition(dial_opts::PeerCondition::Disconnected)
                .build(),
            handler,
        });
    }

    #[inline]
    pub fn remove_node_from_partial_view(&mut self, _peer_id: &PeerId) {
        //self.can_send_peers.remove(peer_id);
    }

    pub fn send_message(&mut self, recv_peer_id: &PeerId, data: impl Into<Vec<u8>>) {
        let message = UnicastMessage {
            source: self.config.local_peer_id.to_bytes(),
            data: data.into(),
            sequence_number: rand::random::<[u8; 20]>().to_vec(),
        };

        // println!("Send_message ok");

        // println!(
        //     "can_send_peers {}",
        //     self.can_send_peers.contains(recv_peer_id)
        // );
        // println!(
        //     "connected_peers {}",
        //     self.connected_peers.contains(recv_peer_id)
        // );
        if self.can_send_peers.contains(recv_peer_id) && self.connected_peers.contains(recv_peer_id)
        {
            // println!("Send_message ok");
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *recv_peer_id,
                    handler: NotifyHandler::Any,
                    event: message,
                });
        }
    }

    pub fn rand_send_message(&mut self, data: impl Into<Vec<u8>>) {
        let message = UnicastMessage {
            source: self.config.local_peer_id.to_bytes(),
            data: data.into(),
            sequence_number: rand::random::<[u8; 20]>().to_vec(),
        };

        let mut r = rand::thread_rng();
        let num = r.gen_range(0..self.can_send_peers.len());
        let recv_peer_id = self.can_send_peers.get(num).unwrap();

        println!("Send to {}", recv_peer_id.to_string());
        if self.can_send_peers.contains(recv_peer_id) && self.connected_peers.contains(recv_peer_id)
        {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *recv_peer_id,
                    handler: NotifyHandler::Any,
                    event: message,
                });
        }
    }
}

impl NetworkBehaviour for Unicast {
    type ConnectionHandler = OneShotHandler<UnicastProtocol, UnicastMessage, InnerMessage>;
    type OutEvent = UnicastEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        //Default::default()
        OneShotHandler::new(
            SubstreamProtocol::new(Default::default(), ()),
            OneShotHandlerConfig {
                keep_alive_timeout: Duration::from_secs(100),
                outbound_substream_timeout: Duration::from_secs(100),
                max_dial_negotiated: 16,
            },
        )
    }

    fn inject_connection_established(
        &mut self,
        id: &PeerId,
        _: &ConnectionId,
        _: &ConnectedPoint,
        _: Option<&Vec<Multiaddr>>,
        other_established: usize,
    ) {
        if other_established > 0 {
            return;
        }

        println!("{} connected!", id.to_string());
        self.connected_peers.insert(*id);
    }

    fn inject_connection_closed(
        &mut self,
        id: &PeerId,
        _: &ConnectionId,
        _: &ConnectedPoint,
        _: Self::ConnectionHandler,
        remaining_established: usize,
    ) {
        if remaining_established > 0 {
            return;
        }

        if self.connected_peers.remove(id) {
            println!("Remove {} successfully.", &id);
        }

        if self.can_send_peers.contains(id) {
            let handler = self.new_handler();
            self.events.push_back(NetworkBehaviourAction::Dial {
                opts: DialOpts::peer_id(*id)
                    .condition(dial_opts::PeerCondition::Disconnected)
                    .build(),
                handler,
            });
        }
    }

    fn inject_event(&mut self, _peer_id: PeerId, _connection: ConnectionId, event: InnerMessage) {
        let event = match event {
            InnerMessage::Rx(event) => event,
            InnerMessage::Sent => return,
        };

        match self.received.test_and_add(&event) {
            Ok(_) => {}
            Err(e @ CuckooError::NotEnoughSpace) => {
                eprintln!(
                    "Message was added to 'received' Cuckoofilter but some \
                     other message was removed as a consequence: {}",
                    e,
                );
            }
        }

        // println!("==================================================");
        // println!("{:?}", &event);

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            UnicastEvent::Message(event),
        ));
    }

    fn poll(
        &mut self,
        _: &mut std::task::Context<'_>,
        _: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}

// Unicast behaviour config
pub struct UnicastConfig {
    pub local_peer_id: PeerId,
}

impl UnicastConfig {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self { local_peer_id }
    }
}
