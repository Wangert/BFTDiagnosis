pub struct State {
    pub fault_tolerance_count: u64,
}

impl State {
    pub fn new() -> Self {
        Self {
            fault_tolerance_count: 1,
        }
    }
}