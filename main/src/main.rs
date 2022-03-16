use cli::args::Args;
use consensus::pbft::{self, node::Node, message::{Request, MessageType, Message}};
use network::peer::Peer;
use structopt::StructOpt;
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::mpsc,
};
use utils::{parse::into_ip4_tcp_multiaddr, coder::{serialize_into_bytes, deserialize_for_bytes}};
use chrono::prelude::*;

#[tokio::main]
async fn main() {
    let args: Args = Args::from_args();
    println!("{:?}", &args);

    let mdns_addr = into_ip4_tcp_multiaddr(args.mdns_addr.as_str(), args.mdns_port);
    let gossipsub_addr = into_ip4_tcp_multiaddr(args.gossipsub_addr.as_str(), args.gossipsub_port);

    let local_peer = Peer::new(mdns_addr, gossipsub_addr);

    let mut node = Node::new(Box::new(local_peer));

    let broadcast_tx = node.executor.broadcast_tx.clone();

    //let (ntx, mut nrx) = mpsc::channel::<String>(5);
    tokio::spawn(async move {
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            match stdin.next_line().await {
                Ok(_) => {
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

                    //message_tx.send(broadcast_msg).await;

                    Peer::broadcast_message(&broadcast_tx, &broadcast_msg).await;
                    //node.pbft_message_handler().await;
                }
                _ => {}
            }
        }
    });

    //tokio::join!(node.network_peer_start(), node.pbft_message_handler());

    node.network_peer_start().await;

    


    // let (tx, mut rx) = mpsc::channel::<String>(5);
    // tokio::spawn(async move {
    //     // Read full lines from stdin
    //     let mut stdin = io::BufReader::new(io::stdin()).lines();

    //     loop {
    //         match stdin.next_line().await {
    //             Ok(Some(s)) => {
    //                 println!("Input msg:{}", &s);
    //                 Peer::broadcast_message(&tx, &s).await;
    //             }
    //             _ => {
    //                 eprintln!("Input message again!");
    //             }
    //         }
    //     }
    // });

    // local_peer.run(&mut rx).await;
}
