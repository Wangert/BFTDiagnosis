use std::error::Error;

use BFTDiagnosis::framework::BFTDiagnosisFramework;
use libp2p::PeerId;
use components::controller::controller::Controller;
use cli::args::Args;
use network::peer::Peer;
use structopt::StructOpt;

use utils::parse::into_ip4_tcp_multiaddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut framework = BFTDiagnosisFramework::new();
    framework.run().await?;
    Ok(())
}

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // let args: Args = Args::from_args();
//     // println!("{:?}", &args);

//     // let is_consensus_node = args.is_consensus_node;
//     let controller_config = BFTDiagnosis::config::read_controller_config();
//     let controller_ip_addr = controller_config.clone().network.unwrap().ip_addr.unwrap();
//     let controller_ip_port = controller_config.network.unwrap().ip_port.unwrap().clone();

//     let swarm_addr = into_ip4_tcp_multiaddr(controller_ip_addr.as_str(), controller_ip_port);
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     let mut node = Controller::new(local_peer, PeerId::random(), controller_ip_port.to_string().as_str());
//     node.peer_start().await?;

//     // if is_consensus_node.eq("true") {
//     //     // let mut node = chain_hotstuff::consensus_node::node::Node::new(Box::new(local_peer), &args.swarm_port.to_string());
//     //     // node.network_peer_start().await?;
//     // } else if is_consensus_node.eq("false") {
//     //     let mut node = Controller::new(local_peer, PeerId::random(), &args.swarm_port.to_string());
//     //     node.peer_start().await?;
//     // } else {
//     //     println!("consensus argument error!");
//     // }

//     Ok(())
// }

//==============================================
// Chain HotStuff protocol

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let args: Args = Args::from_args();
//     println!("{:?}", &args);

//     let is_consensus_node = args.is_consensus_node;
//     let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = chain_hotstuff::consensus_node::node::Node::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = chain_hotstuff::controller_node::node::Node::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }

//==============================================
// Basic HotStuff protocol

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let args: Args = Args::from_args();
//     println!("{:?}", &args);

//     let is_consensus_node = args.is_consensus_node;
//     let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = basic_hotstuff::consensus_node::node::Node::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = basic_hotstuff::controller_node::node::Node::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }

//==============================================
// PBFT protocol

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let args: Args = Args::from_args();
//     println!("{:?}", &args);

//     let is_consensus_node = args.is_consensus_node;
//     let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
//     let local_peer = Peer::new(swarm_addr);

//     // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

//     if is_consensus_node.eq("true") {
//         let mut node = ConsensusNode::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else if is_consensus_node.eq("false") {
//         let mut node = ControllerNode::new(Box::new(local_peer), &args.swarm_port.to_string());
//         node.network_peer_start().await?;
//     } else {
//         println!("consensus argument error!");
//     }

//     Ok(())
// }


