pub struct State {
    pub view: u64,
    pub current_seq_number: u64,
    pub primary: String,
    pub node_count: u64,
    pub fault_tolerance_count: u64,   
}

impl State {
    pub fn new() -> State {
        State { view: 0, current_seq_number: 0, primary: String::from(""), node_count: 4, fault_tolerance_count: 1}
    }
}