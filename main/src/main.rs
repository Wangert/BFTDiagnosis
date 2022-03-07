use network::{
    discovery::MdnsSwarm,
    gossipsub::GossipsubSwarm,
    peer::{run, Peer},
};

#[tokio::main]
async fn main() {
    let mut local_peer = Peer::new("/ip4/10.162.133.179/tcp/51002".parse().unwrap(), 31001);
    let keys = local_peer.get_keys().clone();
    let mut mdns_swarm = MdnsSwarm::new(&keys);

    let mut gossipsub_swarm = GossipsubSwarm::new(&keys);

    let (f1, f2) = run(&mut local_peer, &mut mdns_swarm, &mut gossipsub_swarm).await;

    match (f1, f2) {
        (Ok(_), Ok(_)) => println!("Peer runing successful!"),
        _ => panic!("Peer running error!"),
    }
    //run(&mut local_peer);

    //println!("Hello, world!");
}
