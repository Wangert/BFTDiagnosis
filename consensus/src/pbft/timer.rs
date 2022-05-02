use std::{sync::Arc, time::Duration, collections::HashMap};

use tokio::sync::{Notify, Mutex};

pub struct Timeout {
    pub state: TimeoutState,
    pub duration: Duration,
    pub notify: Arc<Notify>,
}

pub enum TimeoutState {
    Active,
    Inactive,
}

impl Timeout {
    pub fn new(duration: Duration) -> Self {
        Timeout { state: TimeoutState::Inactive, duration, notify: Arc::new(Notify::new()) }
    }

    pub fn start(&mut self) {
        self.state = TimeoutState::Active;
    }

    pub fn stop(&mut self) {
        self.state = TimeoutState::Inactive;
    }
}

pub async fn timeout_tick<F: FnMut()>(duration: u64, notify: Arc<Notify>, mut event_func: F) {
    if let Err(_) = tokio::time::timeout(Duration::from_secs(duration), notify.notified()).await {
        println!("timeout!");
        event_func()
    }
}