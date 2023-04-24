use super::protocol::{
    FloodsubData, FloodsubProtocol, FloodsubMessage, FloodsubSubscription,
    FloodsubSubscriptionAction,};
use super::FloodsubConfig;
use super::topic::Topic;
use std::collections::HashMap;
use std::task::Context;
use std::{
    collections::{hash_map::DefaultHasher, HashSet, VecDeque},
    
    time::Duration,
};
use std::task::{Poll};
use cuckoofilter::{CuckooError, CuckooFilter};
use fnv::FnvHashSet;
use libp2p::swarm::{PollParameters, OneShotHandlerConfig, SubstreamProtocol};
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint,Multiaddr, PeerId},
    swarm::{
        dial_opts::{self, DialOpts},
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler,
    },
    
};
use rand::Rng;
use smallvec::SmallVec;
use std::{iter};

pub struct Floodsub {
    events: VecDeque<
        NetworkBehaviourAction<
            FloodsubEvent,
            OneShotHandler<FloodsubProtocol, FloodsubData, InnerMessage>,
        >,
    >,
    config: FloodsubConfig,
    target_peers: FnvHashSet<PeerId>,
    connected_peers: HashMap<PeerId, SmallVec<[Topic; 8]>>,
    subscribed_topics: SmallVec<[Topic; 16]>,
    received: CuckooFilter<DefaultHasher>,
}

impl Floodsub {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self::from_config(FloodsubConfig::new(local_peer_id))
    }

    pub fn from_config(config: FloodsubConfig) -> Self {
        Floodsub {
            events: VecDeque::new(),
            config,
            target_peers: FnvHashSet::default(),
            connected_peers: HashMap::new(),
            subscribed_topics: SmallVec::new(),
            received: CuckooFilter::new(),
        }
    }

    pub fn add_node_to_partial_view(&mut self, peer_id: PeerId) {
        // Send our topics to this node if we're already connected to it.
        if self.connected_peers.contains_key(&peer_id) {
            for topic in self.subscribed_topics.iter().cloned() {
                self.events
                    .push_back(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: FloodsubData {
                            messages: Vec::new(),
                            subscriptions: vec![FloodsubSubscription {
                                topic,
                                action: FloodsubSubscriptionAction::Subscribe,
                            }],
                        },
                    });
            }
        }

        if self.target_peers.insert(peer_id) {
            let handler = self.new_handler();
            self.events.push_back(NetworkBehaviourAction::Dial {
                opts: DialOpts::peer_id(peer_id)
                    .condition(dial_opts::PeerCondition::Disconnected)
                    .build(),
                handler,
            });
        }

    }

    #[inline]
    pub fn remove_node_from_partial_view(&mut self, peer_id: &PeerId) {
        self.target_peers.remove(peer_id);
    }

    pub fn subscribe(&mut self, topic: Topic) -> bool {
        if self.subscribed_topics.iter().any(|t| t.id() == topic.id()) {
            return false;
        }

        for peer in self.connected_peers.keys() {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer,
                    handler: NotifyHandler::Any,
                    event: FloodsubData {
                        messages: Vec::new(),
                        subscriptions: vec![FloodsubSubscription {
                            topic: topic.clone(),
                            action: FloodsubSubscriptionAction::Subscribe,
                        }],
                    },
                });
        }

        self.subscribed_topics.push(topic);
        true
    }

    pub fn unsubscribe(&mut self, topic: Topic) -> bool {
        let pos = match self.subscribed_topics.iter().position(|t| *t == topic) {
            Some(pos) => pos,
            None => return false,
        };

        self.subscribed_topics.remove(pos);

        for peer in self.connected_peers.keys() {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer,
                    handler: NotifyHandler::Any,
                    event: FloodsubData {
                        messages: Vec::new(),
                        subscriptions: vec![FloodsubSubscription {
                            topic: topic.clone(),
                            action: FloodsubSubscriptionAction::Unsubscribe,
                        }],
                    },
                });
        }

        true
    }

    pub fn publish(&mut self, topic: impl Into<Topic>, data: impl Into<Vec<u8>>) {
        self.publish_many(iter::once(topic), data)
    }

    pub fn publish_any(&mut self, topic: impl Into<Topic>, data: impl Into<Vec<u8>>) {
        self.publish_many_any(iter::once(topic), data)
    }

    pub fn publish_many(
        &mut self,
        topic: impl IntoIterator<Item = impl Into<Topic>>,
        data: impl Into<Vec<u8>>,
    ) {
        self.publish_many_inner(topic, data, true)
    }

    pub fn publish_many_any(
        &mut self,
        topic: impl IntoIterator<Item = impl Into<Topic>>,
        data: impl Into<Vec<u8>>,
    ) {
        self.publish_many_inner(topic, data, false)
    }

    fn publish_many_inner(
        &mut self,
        topic: impl IntoIterator<Item = impl Into<Topic>>,
        data: impl Into<Vec<u8>>,
        check_self_subscriptions: bool,
    ) {
        // println!("111");
        let message = FloodsubMessage {
            source: self.config.local_peer_id.to_bytes(),
            data: data.into(),
            // If the sequence numbers are predictable, then an attacker could flood the network
            // with packets with the predetermined sequence numbers and absorb our legitimate
            // messages. We therefore use a random number.
            sequence_number: rand::random::<[u8; 20]>().to_vec(),
            topics: topic.into_iter().map(Into::into).collect(),
        };
        // println!("222");
        let self_subscribed = self
            .subscribed_topics
            .iter()
            .any(|t| message.topics.iter().any(|u| t == u));
        if self_subscribed {
            // println!("333");
            if let Err(e @ CuckooError::NotEnoughSpace) = self.received.add(&message) {
                eprintln!(
                    "Message was added to 'received' Cuckoofilter but some \
                     other message was removed as a consequence: {}",
                    e,
                );
            }
            if self.config.allow_self_origin {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    FloodsubEvent::Message(message.clone()),
                ));
            }
        }
        // Don't publish the message if we have to check subscriptions
        // and we're not subscribed ourselves to any of the topics.
        // if check_self_subscriptions && !self_subscribed {
        //     return;
        // }

        // Send to peers we know are subscribed to the topic.
        for (peer_id, sub_topic) in self.connected_peers.iter() {

            println!("!!!peed_id:{:?}",peer_id.to_string());
            
            // Peer must be in a communication list.
            if !self.target_peers.contains(peer_id) {
                continue;
            }
            // println!("peer id L{:?}.sub:{:?}",peer_id.clone(),sub_topic.clone());
            // Peer must be subscribed for the topic.
            if !sub_topic
                .iter()
                .any(|t| message.topics.iter().any(|u| t == u))
            {
                continue;
            }
            
            
            // println!("{:?}",message.clone().data);
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer_id,
                    handler: NotifyHandler::Any,
                    event: FloodsubData {
                        subscriptions: Vec::new(),
                        messages: vec![message.clone()],
                    },
                });
            // println!("向节点{}发送Notify",peer_id.clone().to_string());
        }
    }

}

impl NetworkBehaviour for Floodsub {
    type ConnectionHandler = OneShotHandler<FloodsubProtocol, FloodsubData, InnerMessage>;
    type OutEvent = FloodsubEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        // Default::default()
        OneShotHandler::new(
            SubstreamProtocol::new(Default::default(), ()),
            OneShotHandlerConfig {
                keep_alive_timeout: Duration::from_secs(100),
                outbound_substream_timeout: Duration::from_secs(100),
                max_dial_negotiated: 50,
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
            // We only care about the first time a peer connects.
            return;
        }

        // We need to send our subscriptions to the newly-connected node.
        if self.target_peers.contains(id) {
            for topic in self.subscribed_topics.iter().cloned() {
                self.events
                    .push_back(NetworkBehaviourAction::NotifyHandler {
                        peer_id: *id,
                        handler: NotifyHandler::Any,
                        event: FloodsubData {
                            messages: Vec::new(),
                            subscriptions: vec![FloodsubSubscription {
                                topic,
                                action: FloodsubSubscriptionAction::Subscribe,
                            }],
                        },
                    });
            }
        }
        self.connected_peers.insert(*id, SmallVec::new());
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
            // we only care about peer disconnections
            return;
        }

        let was_in = self.connected_peers.remove(id);
        debug_assert!(was_in.is_some());

        // We can be disconnected by the remote in case of inactivity for example, so we always
        // try to reconnect.
        if self.target_peers.contains(id) {
            let handler = self.new_handler();
            self.events.push_back(NetworkBehaviourAction::Dial {
                opts: DialOpts::peer_id(*id)
                    .condition(dial_opts::PeerCondition::Disconnected)
                    .build(),
                handler,
            });
        }
    }

    fn inject_event(
        &mut self,
        propagation_source: PeerId,
        _connection: ConnectionId,
        event: InnerMessage,
    ) {
        
        // We ignore successful sends or timeouts.
        let event = match event {
            InnerMessage::Rx(event) => {
                // println!("Get inject_event");
                // if let Some(data) = event.clone().messages.pop() {
                //     println!("{:?}",data.data);
                // }
                
                event
            } ,
            InnerMessage::Sent => {
                return
            } ,
        };

        // Update connected peers topics
        for subscription in event.subscriptions {
            let remote_peer_topics = self.connected_peers
                .get_mut(&propagation_source)
                .expect("connected_peers is kept in sync with the peers we are connected to; we are guaranteed to only receive events from connected peers; QED");
            match subscription.action {
                FloodsubSubscriptionAction::Subscribe => {
                    if !remote_peer_topics.contains(&subscription.topic) {
                        remote_peer_topics.push(subscription.topic.clone());
                    }
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        FloodsubEvent::Subscribed {
                            peer_id: propagation_source,
                            topic: subscription.topic,
                        },
                    ));
                }
                FloodsubSubscriptionAction::Unsubscribe => {
                    if let Some(pos) = remote_peer_topics
                        .iter()
                        .position(|t| t == &subscription.topic)
                    {
                        remote_peer_topics.remove(pos);
                    }
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        FloodsubEvent::Unsubscribed {
                            peer_id: propagation_source,
                            topic: subscription.topic,
                        },
                    ));
                }
            }
        }

        // List of messages we're going to propagate on the network.
        let mut rpcs_to_dispatch: Vec<(PeerId, FloodsubData)> = Vec::new();

        for message in event.messages {
            // println!("收到message");
            // println!("{:?}",&message.data);
            // Use `self.received` to skip the messages that we have already received in the past.
            // Note that this can result in false positives.
            match self.received.test_and_add(&message) {
                Ok(true) => {
                    // println!("新消息!");
                }         // Message  was added.
                Ok(false) => {
                    // println!("旧消息!");
                    continue
                } , // Message already existed.
                Err(e @ CuckooError::NotEnoughSpace) => {
                    // Message added, but some other removed.
                    eprintln!(
                        "Message was added to 'received' Cuckoofilter but some \
                         other message was removed as a consequence: {}",
                        e,
                    );
                }
            }
            // println!("处理message");

            // Add the message to be dispatched to the user.
            if self
                .subscribed_topics
                .iter()
                .any(|t| message.topics.iter().any(|u| t == u))
            {
                let event = FloodsubEvent::Message(message.clone());
                self.events
                    .push_back(NetworkBehaviourAction::GenerateEvent(event));
            }

            // Propagate the message to everyone else who is subscribed to any of the topics.
            for (peer_id, subscr_topics) in self.connected_peers.iter() {
                if peer_id == &propagation_source {
                    continue;
                }

                // Peer must be in a communication list.
                if !self.target_peers.contains(peer_id) {
                    continue;
                }

                // Peer must be subscribed for the topic.
                if !subscr_topics
                    .iter()
                    .any(|t| message.topics.iter().any(|u| t == u))
                {
                    continue;
                }

                if let Some(pos) = rpcs_to_dispatch.iter().position(|(p, _)| p == peer_id) {
                    rpcs_to_dispatch[pos].1.messages.push(message.clone());
                } else {
                    rpcs_to_dispatch.push((
                        *peer_id,
                        FloodsubData {
                            subscriptions: Vec::new(),
                            messages: vec![message.clone()],
                        },
                    ));
                }
            }
        }

        for (peer_id, data) in rpcs_to_dispatch {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler: NotifyHandler::Any,
                    event: data,
                });
        }
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }
        
        Poll::Pending
    }
}


#[derive(Debug)]
pub enum InnerMessage {
    /// We received an RPC from a remote.
    Rx(FloodsubData),
    /// We successfully sent an RPC request.
    Sent,
}

impl From<FloodsubData> for InnerMessage {
    #[inline]
    fn from(rpc: FloodsubData) -> InnerMessage {
        InnerMessage::Rx(rpc)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Sent
    }
}

#[derive(Debug)]
pub enum FloodsubEvent {
    /// A message has been received.
    Message(FloodsubMessage),

    /// A remote subscribed to a topic.
    Subscribed {
        /// Remote that has subscribed.
        peer_id: PeerId,
        /// The topic it has subscribed to.
        topic: Topic,
    },

    /// A remote unsubscribed from a topic.
    Unsubscribed {
        /// Remote that has unsubscribed.
        peer_id: PeerId,
        /// The topic it has subscribed from.
        topic: Topic,
    },
}