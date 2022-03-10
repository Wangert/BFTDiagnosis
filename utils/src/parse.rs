use libp2p::{Multiaddr, multiaddr::Protocol};

pub fn into_ip4_tcp_multiaddr(ip_addr: &str, port: u16) -> Multiaddr {
    let mut prefix = String::from("/ip4/");
    prefix.push_str(ip_addr);
    let mut addr: Multiaddr = prefix.parse().unwrap();
    addr.push(Protocol::Tcp(port));

    addr
}