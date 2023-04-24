use libp2p::PeerId;

pub mod protocol;
pub mod behaviour;
pub mod topic;

/// Configuration options for the Floodsub protocol.
#[derive(Debug, Clone)]
pub struct FloodsubConfig {
    /// Peer id of the local node. Used for the source of the messages that we publish.
    pub local_peer_id: PeerId,

    /// `true` if messages published by local node should be propagated as messages received from
    /// the network, `false` by default.
    pub allow_self_origin: bool,
}

impl FloodsubConfig {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            allow_self_origin: true,
        }
    }
}