pub struct State {
    pub view: u64,
    pub current_seq_number: u64,
    pub primary: String,    
}

impl State {
    pub fn new() -> State {
        State { view: 0, current_seq_number: 0, primary: String::from("") }
    }
}