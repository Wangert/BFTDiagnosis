use std::{error::Error};


use cli::args::Args;
use consensus::pbft::{
    node::ConsensusNode, controller_node::ControllerNode,
};
use network::peer::Peer;
use structopt::StructOpt;

use utils::{
    parse::into_ip4_tcp_multiaddr,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::from_args();
    println!("{:?}", &args);

    let is_consensus_node = args.is_consensus_node;
    let swarm_addr = into_ip4_tcp_multiaddr(args.swarm_addr.as_str(), args.swarm_port);
    let local_peer = Peer::new(swarm_addr);


    // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

    if is_consensus_node.eq("true") {
        let mut node = ConsensusNode::new(Box::new(local_peer), &args.swarm_port.to_string());
        node.network_peer_start().await?;
    } else if is_consensus_node.eq("false") {
        let mut node = ControllerNode::new(Box::new(local_peer), &args.swarm_port.to_string());
        node.network_peer_start().await?;
    } else {
        println!("consensus argument error!");
    }

    Ok(())
}
