use std::collections::HashSet;

use super::message::Message;

pub struct LocalLogs {
    pub messages: HashSet<Message>,
}

impl LocalLogs {
    pub fn new() -> LocalLogs {
        let messages: HashSet<Message> = HashSet::new();
        LocalLogs { messages }
    }
}