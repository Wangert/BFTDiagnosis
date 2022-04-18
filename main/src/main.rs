use std::{error::Error, sync::Arc};

use chrono::prelude::*;
use cli::args::Args;
use consensus::pbft::{
    self,
    message::{Message, MessageType, Request},
    node::Node,
};
use network::peer::Peer;
use structopt::StructOpt;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, Mutex},
};
use utils::{
    coder::{deserialize_for_bytes, serialize_into_bytes},
    parse::into_ip4_tcp_multiaddr,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::from_args();
    println!("{:?}", &args);

    let is_consensus_node = args.is_consensus_node;
    let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
    let local_peer = Peer::new(swarm_addr);


    let mut node = Node::new(Box::new(local_peer));

    if is_consensus_node.eq("true") {
        node.network_peer_start(true).await?;
    } else if is_consensus_node.eq("false") {
        node.network_peer_start(false).await?;
    } else {
        println!("consensus argument error!");
    }

    Ok(())
}
