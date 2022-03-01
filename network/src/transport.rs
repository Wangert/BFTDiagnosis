use std::io::Error;

use futures::executor::block_on;
use libp2p::{
    core::{self, muxing::StreamMuxerBox, transport::Boxed},
    dns,
    identity::Keypair,
    mplex,
    noise,
    tcp,
    websocket, yamux, PeerId, Transport,
};

pub struct CMTTransport(pub Boxed<(PeerId, StreamMuxerBox)>);

impl CMTTransport {
    pub fn new(keys: &Keypair) -> CMTTransport {
        let future = create_transport(keys);
        let transport_result = block_on(future);
        match transport_result {
            Ok(t) => CMTTransport(t),
            Err(e) => panic!("【network_transport】:{:?}", e),
        }
    }
}

pub async fn create_transport(
    keypair: &Keypair,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>, Error> {
    let transport = {
        let tcp = tcp::TcpConfig::new().nodelay(true);
        let dns_tcp = dns::DnsConfig::system(tcp).await?;
        let ws_dns_tcp = websocket::WsConfig::new(dns_tcp.clone());
        dns_tcp.or_transport(ws_dns_tcp)
    };

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(keypair)
        .expect("Signing libp2p-noise static DH keypair failed.");

    Ok(transport
        .upgrade(core::upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(core::upgrade::SelectUpgrade::new(
            yamux::YamuxConfig::default(),
            mplex::MplexConfig::default(),
        ))
        .timeout(std::time::Duration::from_secs(20))
        .boxed())
}
