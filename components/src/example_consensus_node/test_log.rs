use crate::behaviour::ProtocolLogsReadBehaviour;

pub struct TestLog {

}

impl Default for TestLog {
    fn default() -> Self {
        Self {  }
    }
}

impl ProtocolLogsReadBehaviour for TestLog {
    fn get_ledger(&mut self) {
        todo!()
    }

    fn get_current_leader(&self) {
        todo!()
    }
}