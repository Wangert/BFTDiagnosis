use std::error::Error;

use BFTDiagnosis::framework::BFTDiagnosisFramework;
use cli::args::Args;
use components::protocol_actuator::ProtocolActuator;
use consensus::chain_hotstuff;
// use consensus::{basic_hotstuff, chain_hotstuff, pbft};
use network::peer::Peer;
use protocols::pbft::protocol::PBFTProtocol;
use protocols::basic_hotstuff::protocol::BasicHotstuffProtocol;
use protocols::chain_hotstuff::protocol::ChainHotstuffProtocol;
use structopt::StructOpt;
use utils::parse::into_ip4_tcp_multiaddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut framework: BFTDiagnosisFramework<_> = BFTDiagnosisFramework::new();    

    let consensus_node: ProtocolActuator<PBFTProtocol> =
        ProtocolActuator::new();
    framework.set_consensus_node(consensus_node);
    
    framework.run().await?;
    Ok(())
}

// ==============================================
// Basic HotStuff protocol
// Usage: cargo run -- --consensus 6666 true （共识节点）
//        cargo run -- --consensus 6666 false（控制节点）
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // let args: Args = Args::from_args();
//     // println!("{:?}", &args);
//     let port = std::env::args().nth(2).expect("no port");
//     let mut v: Vec<u16> = port.encode_utf16().collect();
//     println!("{}",port);
//     v.push(0);
//     let is_consensus_node = std::env::args().nth(3).expect("no consensus_node"); 
//     let swarm_addr = into_ip4_tcp_multiaddr("10.162.208.232", v.pop().unwrap());
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = basic_hotstuff::consensus_node::node::Node::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = basic_hotstuff::controller_node::node::Node::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }

// Chain Hotstuff
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // let args: Args = Args::from_args();
//     // println!("{:?}", &args);
//     let port = std::env::args().nth(2).expect("no port");
//     let mut v: Vec<u16> = port.encode_utf16().collect();
//     println!("{}",port);
//     v.push(0);
//     let is_consensus_node = std::env::args().nth(3).expect("no consensus_node"); 
//     let swarm_addr = into_ip4_tcp_multiaddr("10.162.208.232", v.pop().unwrap());
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = chain_hotstuff::consensus_node::node::Node::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = chain_hotstuff::controller_node::node::Node::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }

// PBFT
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // let args: Args = Args::from_args();
//     // println!("{:?}", &args);
//     let port = std::env::args().nth(2).expect("no port");
//     let mut v: Vec<u16> = port.encode_utf16().collect();
//     println!("{}",port);
//     v.push(0);
//     let is_consensus_node = std::env::args().nth(3).expect("no consensus_node"); 
//     let swarm_addr = into_ip4_tcp_multiaddr("10.162.208.232", v.pop().unwrap());
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = pbft::node::ConsensusNode::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = pbft::controller_node::ControllerNode::new(Box::new(local_peer), port.clone().as_str());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }