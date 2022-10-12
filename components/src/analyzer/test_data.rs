use std::collections::HashMap;

use libp2p::PeerId;

use crate::message::{ConsensusStartData, ConsensusEndData};

pub struct TestData {
    consensus_start_data: HashMap<PeerId, ConsensusStartData>,
    consensus_end_data: HashMap<PeerId, ConsensusEndData>,
}