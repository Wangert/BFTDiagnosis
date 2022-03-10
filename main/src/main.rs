use cli::args::Args;
use network::peer::Peer;
use structopt::StructOpt;
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::mpsc,
};
use utils::parse::into_ip4_tcp_multiaddr;

use core::blockchain;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let args: Args = Args::from_args();
    println!("{:?}", &args);

    let mdns_addr = into_ip4_tcp_multiaddr(args.mdns_addr.as_str(), args.mdns_port);
    let gossipsub_addr = into_ip4_tcp_multiaddr(args.gossipsub_addr.as_str(), args.gossipsub_port);

    let mut local_peer = Peer::new(mdns_addr, gossipsub_addr);

    let (tx, mut rx) = mpsc::channel::<String>(5);
    tokio::spawn(async move {
        // Read full lines from stdin
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            match stdin.next_line().await {
                Ok(Some(s)) => {
                    println!("Input msg:{}", &s);
                    Peer::broadcast_message(&tx, &s).await;
                }
                _ => {
                    eprintln!("Input message again!");
                }
            }
        }
    });

    local_peer.run(&mut rx).await;

    // let mut local_peer = Peer::new("/ip4/10.162.133.179/tcp/51002".parse().unwrap(), 31001);
    // let keys = local_peer.get_keys().clone();
    // let mut mdns_swarm = MdnsSwarm::new(&keys);

    // let mut gossipsub_swarm = GossipsubSwarm::new(&keys);

    // let (f1, f2) = run(&mut local_peer, &mut mdns_swarm, &mut gossipsub_swarm).await;

    // match (f1, f2) {
    //     (Ok(_), Ok(_)) => println!("Peer runing successful!"),
    //     _ => panic!("Peer running error!"),
    // }
    //run(&mut local_peer);

    //println!("Hello, world!");
    let mut bc = blockchain::BlockChain::new_blockchain();

    println!("第一轮共识进行中。。。。。。。。。");
    thread::sleep(Duration::from_secs(5));
    bc.add_block(String::from("aa"));

    println!("");
    println!("第二轮共识进行中。。。。。。。。。");
    thread::sleep(Duration::from_secs(5));
    bc.add_block("bb".to_string());
}
