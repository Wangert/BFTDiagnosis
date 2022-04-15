use std::{error::Error, sync::Arc};

use cli::args::Args;
use consensus::pbft::{self, message::{Request, MessageType, Message}, node::Node};
use network::peer::Peer;
use structopt::StructOpt;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    sync::{mpsc, Mutex}, net::TcpStream,
};
use utils::{parse::into_ip4_tcp_multiaddr, coder::{serialize_into_bytes, deserialize_for_bytes}};
use chrono::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let args: Args = Args::from_args();
    println!("{:?}", &args);

    // let mdns_addr = into_ip4_tcp_multiaddr(args.mdns_addr.as_str(), args.mdns_port);
    // let gossipsub_addr = into_ip4_tcp_multiaddr(args.gossipsub_addr.as_str(), args.gossipsub_port);

    let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
    let local_peer = Peer::new(swarm_addr);

    //local_peer.swarm_start().await?;
    //local_peer.message_handler_start().await?;

    let mut node = Node::new(Box::new(local_peer));

    let broadcast_tx = node.executor.broadcast_tx.clone();

    let arc_connected_nodes = node.connected_nodes.clone();
    let arc_current_leaders = node.current_leader.clone();
    // //let (ntx, mut nrx) = mpsc::channel::<String>(5);
    tokio::spawn(async move {
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            match stdin.next_line().await {
                Ok(Some(peer)) => {
                    println!("Peer: {}", peer);

                    let peer_id = arc_connected_nodes.lock().await.get(&peer).unwrap().clone();
                    arc_current_leaders.lock().await.clear();
                    arc_current_leaders.lock().await.push(peer_id);
                    let request = Request {
                        operation: String::from("operation"),
                        client_addr: String::from("client_addr"),
                        timestamp: Local::now().timestamp().to_string(),
                        signature: String::from("signature"),
                    };
                
                    let msg = Message {
                        msg_type: MessageType::Request(request),
                    };
                
                    let serialized_msg = serialize_into_bytes(&msg);
                    let broadcast_msg = std::str::from_utf8(&serialized_msg).unwrap();
            
                    let deserialized_msg: Message = deserialize_for_bytes(broadcast_msg.as_bytes());

                    println!("Deserialzed_msg: {:?}", &deserialized_msg);

                    broadcast_tx.send(broadcast_msg.to_string()).await.unwrap();

                }
                _ => {}
            }
        }
    });

    node.network_peer_start().await?;

    Ok(())

}
