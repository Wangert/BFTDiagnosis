use components::behaviour::NodeStateUpdateBehaviour;

pub struct TestState {

}

impl Default for TestState {
    fn default() -> Self {
        Self {  }
    }
}

impl NodeStateUpdateBehaviour for TestState {
    fn update_consensus_node_count(&mut self, _count: usize) {
        todo!()
    }
}