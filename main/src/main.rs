use cli::args::Args;
use network::peer::Peer;
use structopt::StructOpt;
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::mpsc,
};
use utils::parse::into_ip4_tcp_multiaddr;

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
}
