use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubMessage, MessageAuthenticity,
        MessageId, ValidationMode,
    },
    identity::Keypair,
    mdns::{Mdns, MdnsConfig},
    Multiaddr, PeerId, Swarm,
};

use crate::{
    p2p_protocols::{base_behaviour::BaseBehaviour, unicast::behaviour::Unicast},
    transport::create_base_transport,
};

// mdns + unicast + gossipsub swarm
pub struct BaseSwarm {
    pub swarm: Option<Swarm<BaseBehaviour>>,
}

impl BaseSwarm {
    pub fn new() -> Self {
        Self { swarm: None }
    }

    // build swarm
    pub async fn build(
        &mut self,
        peer_id: PeerId,
        keypair: Keypair,
        _is_consensus_node: bool,
    ) -> Result<(), Box<dyn Error>> {
        // create transport
        let transport = create_base_transport(&keypair).await?;

        // create unicast
        let unicast = Unicast::new(peer_id.clone());

        // create gossipsub
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            let mut msg_hash_vec = if let Some(peer_id) = message.source {
                peer_id.to_bytes()
            } else {
                vec![]
            };

            // let mut t = Local::now().to_string().as_bytes().to_vec();
            let mut data = message.data.clone();
            msg_hash_vec.append(&mut data);
            // msg_hash_vec.append(&mut t);
            msg_hash_vec.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(5))
            .validation_mode(ValidationMode::Permissive)
            .message_id_fn(message_id_fn)
            //.allow_self_origin(true)
            .build()
            .expect("Valid config");

        // let mut gossipsub: Gossipsub = Gossipsub::new(
        //     MessageAuthenticity::Signed(keypair.clone()),
        //     gossipsub_config,
        // )
        // .expect("Gossipsub correct configuration");

        let gossipsub: Gossipsub = Gossipsub::new(
            MessageAuthenticity::Author(peer_id.clone()),
            gossipsub_config,
        )
        .expect("Gossipsub correct configuration");

        // create mdns
        let mdns = Mdns::new(MdnsConfig::default()).await?;

        // create BaseBehaviour
        let base_behaviour = BaseBehaviour {
            unicast,
            gossipsub,
            mdns,
        };

        // create base swarm
        let swarm = Swarm::new(transport, base_behaviour, peer_id);
        self.swarm = Some(swarm);

        Ok(())
    }

    pub fn start(&mut self, multiaddr: Multiaddr) -> Result<(), Box<dyn Error>> {
        let swarm = if let Some(swarm) = &mut self.swarm {
            swarm
        } else {
            panic!("【Base_Swarm】:Not built base_swarm");
        };

        swarm.listen_on(multiaddr)?;

        Ok(())
    }
}
