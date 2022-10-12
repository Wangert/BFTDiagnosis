use libp2p::PeerId;

pub struct ExtraInfo {
    pub controller_node_id: PeerId,
    pub analysis_node_id: PeerId,
}