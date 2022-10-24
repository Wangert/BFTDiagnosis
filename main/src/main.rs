use std::error::Error;

use BFTDiagnosis::framework::BFTDiagnosisFramework;
use components::protocol_actuator::ProtocolActuator;
use protocols::pbft::protocol::PBFTProtocol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut framework: BFTDiagnosisFramework<_> = BFTDiagnosisFramework::new();    

    let consensus_node: ProtocolActuator<PBFTProtocol> =
        ProtocolActuator::new();
    framework.set_consensus_node(consensus_node);
  
    framework.run().await?;
    Ok(())
}

