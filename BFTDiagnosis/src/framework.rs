use cli::{client::{Client, ClientType}, cmd::rootcmd::CMD};
use libp2p::PeerId;
use network::peer::Peer;
use node::{analyzer::analyzer::Analyzer, controller::controller::Controller};
use utils::parse::into_ip4_tcp_multiaddr;

use std::error::Error;
use crate::config::{read_controller_config, read_analyzer_config, read_bft_diagnosis_config};

pub struct BFTDiagnosisFramework {
    // pub controller: Controller,
    // pub analyzer: Analyzer,
    client: Client,
}

impl BFTDiagnosisFramework {
    pub fn new() -> Self {
        let cmd_matches = CMD.clone().get_matches();
        let mut client = Client::new(cmd_matches);

        Self { client }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        if self.client.arg_matches().is_present("controller") {
            println!("Controller");
            let controller_config = read_controller_config();
            let controller_ip_addr = controller_config.clone().network.unwrap().ip_addr.unwrap();
            let controller_ip_port = controller_config.network.unwrap().ip_port.unwrap().clone();

            let swarm_addr =
                into_ip4_tcp_multiaddr(controller_ip_addr.as_str(), controller_ip_port);
            let local_peer = Peer::new(swarm_addr);

            // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

            let mut node = Controller::new(
                local_peer,
                controller_ip_port.to_string().as_str(),
            );

            let bft_diagnosis_config = read_bft_diagnosis_config();

            println!("youqu:{:#?}", &bft_diagnosis_config);


            node.configure(bft_diagnosis_config);

            let args_sender = node.args_sender();
            self.client.run(args_sender, ClientType::Controller);
            node.peer_start().await?;
        } else if self.client.arg_matches().is_present("analyzer") {
            println!("Analyzer");
            let analyzer_config = read_analyzer_config();
            let analyzer_ip_addr = analyzer_config.clone().network.unwrap().ip_addr.unwrap();
            let analyzer_ip_port = analyzer_config.network.unwrap().ip_port.unwrap().clone();

            let swarm_addr =
                into_ip4_tcp_multiaddr(analyzer_ip_addr.as_str(), analyzer_ip_port);
            let local_peer = Peer::new(swarm_addr);

            // let mut node = Node::new(Box::new(local_peer), &args.swarm_port.to_string());

            let mut node = Analyzer::new(
                local_peer,
                PeerId::random(),
                analyzer_ip_port.to_string().as_str(),
            );

            let args_sender = node.args_sender();
            self.client.run(args_sender, ClientType::Analyzer);
            node.peer_start().await?;
        }

        Ok(())
    }
}
