use libp2p::PeerId;

pub struct ExtraInfo {
    controller_node_id: PeerId,
    analysis_node_id: PeerId,
}