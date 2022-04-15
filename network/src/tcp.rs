use std::num::NonZeroI64;

use futures::executor::block_on;
use tokio::net::TcpListener;

pub struct Tcp {
    pub listener: Option<TcpListener>,
}

impl Tcp {
    pub fn new(address: &str) -> Tcp {
        let listener_future = TcpListener::bind(address);
        let listener = block_on(listener_future);
        match listener {
            Ok(l) => {
                Tcp { listener: Some(l) }
            }
            Err(e) => {
                eprintln!("tcp server start failed: {:?}", e);
                Tcp { listener: None }
            }
        }
    }
}