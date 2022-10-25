use cli::{
    client::{Client, ClientType},
    cmd::rootcmd::CMD,
};

use network::peer::Peer;
use components::{
    analyzer::analyzer::Analyzer,
    protocol_actuator::ProtocolActuator,
    controller::controller::Controller, behaviour::ProtocolBehaviour,
};
use utils::parse::into_ip4_tcp_multiaddr;

use crate::config::{read_analyzer_config, read_bft_diagnosis_config, read_controller_config};
use std::error::Error;

pub struct BFTDiagnosisFramework<TProtocol> 
where TProtocol: Default + ProtocolBehaviour,
{
    client: Client,
    consensus_node: Option<ProtocolActuator<TProtocol>>,
}

impl<TProtocol> BFTDiagnosisFramework<TProtocol> 
where TProtocol: Default + ProtocolBehaviour,
{
    pub fn new() -> Self {
        let cmd_matches = CMD.clone().get_matches();
        let client = Client::new(cmd_matches);

        Self { client, consensus_node: None }
    }

    pub fn set_consensus_node(&mut self, consensus_node: ProtocolActuator<TProtocol>) {
        self.consensus_node = Some(consensus_node)
    }

    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    pub fn consensus_node(&mut self) -> &mut ProtocolActuator<TProtocol> {
        if let Some(node) = self.consensus_node.as_mut() {
            node
        } else {
            panic!("Please set consensus node!");
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        if self.client.arg_matches().is_present("controller") {
            println!("Controller");
            let controller_config = read_controller_config();
            let controller_ip_addr = controller_config.clone().network.unwrap().ip_addr.unwrap();
            let controller_ip_port = controller_config.clone().network.unwrap().ip_port.unwrap();
            let conspire_request_send_duration = controller_config
                .clone()
                .extra
                .unwrap()
                .conspire_request_send_duration
                .unwrap();

            let swarm_addr =
                into_ip4_tcp_multiaddr(controller_ip_addr.as_str(), controller_ip_port);
            let local_peer = Peer::new(swarm_addr);

            let mut node = Controller::new(
                local_peer,
                controller_ip_port.to_string().as_str(),
                conspire_request_send_duration,
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

            println!("{:#?}", &analyzer_config);
            let analyzer_ip_addr = analyzer_config.clone().network.unwrap().ip_addr.unwrap();
            let analyzer_ip_port = analyzer_config
                .clone()
                .network
                .unwrap()
                .ip_port
                .unwrap()
                .clone();

            let performance_test_duration = analyzer_config
                .clone()
                .execution
                .unwrap()
                .performance_duration
                .unwrap();
            let performance_test_internal = analyzer_config
                .clone()
                .execution
                .unwrap()
                .performance_internal
                .unwrap();
            let crash_test_duration = analyzer_config
                .clone()
                .execution
                .unwrap()
                .crash_duration
                .unwrap();
            let malicious_test_duration = analyzer_config
                .execution
                .unwrap()
                .malicious_duration
                .unwrap();

            let swarm_addr = into_ip4_tcp_multiaddr(analyzer_ip_addr.as_str(), analyzer_ip_port);
            let local_peer = Peer::new(swarm_addr);

            let mut node = Analyzer::new(
                local_peer,
                performance_test_duration,
                performance_test_internal,
                crash_test_duration,
                malicious_test_duration,
            );

            let args_sender = node.args_sender();
            self.client.run(args_sender, ClientType::Analyzer);
            node.peer_start().await?;
        } else if self.client.arg_matches().is_present("consensus") {
            println!("OK");
            if let Some(values) = self.client.arg_matches().values_of("consensus") {
                let msg: Vec<_> = values.collect();

                println!("{}", msg[0]);

                let swarm_addr = into_ip4_tcp_multiaddr(
                    msg[0],
                    msg[1].parse::<i32>().unwrap().try_into().unwrap(),
                );
                let local_peer = Peer::new(swarm_addr);
                println!(
                    "\nConsensus node has generated.PeerId is : {:?}",
                    local_peer.id
                );

                let is_leader = msg[2].parse::<bool>().unwrap();
                println!("{}", is_leader);

                self.consensus_node().set_peer(local_peer);

                self.client().consensus_run();
                self.consensus_node().network_start(is_leader).await?;
            }
        }

        Ok(())
    }
}
