# BFTDiagnosis
A universal and flexible testing framework of partial synchronous BFT protocols based on Rust language.

The universality and flexibility of BFTDiagnosis benefit from the design of three key components: Controller, Protocol Actuator and Analyzer. To support the behaviour diversity of different protocols, BFTDiagnosis sets a node mode with behaviour-triggering in Protocol Actuator, and uses Controller to control behaviours of protocols.

## Key Components
### Controller
Controller manages the working process of Protocol Actuator and Analyzer. When Controller is initialized, test items are generated based on a configuration
file. When each test item is executed, Controller configures the state of Protocol Actuator and Analyzer, then executes a test item. Before proceeding to the next round of test, Controller waits for a response from Analyzer at the end of this round of test.
### Analyzer
Analyzer collects and analyzes protocol running data. Analyzer receives the protocol running data from consensus nodes and stores it in a database. Then,
Analyzer calculates and analyzes the data in the light of the current test item, and outputs test results.
### Protocol Actuator
A container that runs partial synchronous BFT protocols. In initial state, there is no protocol in ProtocolActuator, so users need to implement
protocols based on custom-interfaces of Protocol Actuator. Protocol Actuator with protocol execution logic is a runnable consensus node. The consensus node can conduct the required behaviours in protocol testing process by interactive instructions from Controller. Meanwhile continuously deliver protocol running data to Analyzer during the test execution

## Usage
### 1.Protocol customization phase
In this phase, users need to integrate protocols into Protocol Actuator. Protocol Actuator in initial state has Default Interfaces for component interaction and Unfinished Custom Interfaces for protocol integration, but no protocol execution logic. Therefore, users need to implement the protocol based on the Unfinished Custom Interfaces to transform Protocol Actuator into a Runnable consensus node.

Details:
Users need to implement ```ProtocolBehaviour``` trait to custom a unique protocol and can also refer to the ```/components/src/example_consensus_node/protocol``` for the specific implementation.Follows the functions.

PS: We have five types of Partial Synchronous BFT Protocols implemented in the ```/protocols``` containing Three types of PBFT and Two types of HotStuff.


``` dart
    ///
    /// init the protocol's timeout notify by the param (timeout_notify)
    ///
    fn init_timeout_notify(&mut self, timeout_notify: Arc<Notify>)
    ///
    /// In addition to the default network startup and key distribution.
    /// the protocol may have additional initiators that can be added to this method.
    ///  
    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
    ) -> PhaseState
    ///
    /// Receives a consensus request from the Controller node
    ///  
    fn receive_consensus_requests(&mut self, requests: Vec<Request>)
    ///
    /// Consensus protocol's message handler,it handles the logic of the consensus protocol.
    /// 
    fn consensus_protocol_message_handler(&mut self, _msg: &[u8],current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>) -> PhaseState
    /// 
    /// Get the phase_num the consensus protocol is processing in.
    ///  
    fn get_current_phase(&mut self, _msg: &[u8]) -> u8
    ///
    /// Reset all the running data of the consensus protocol.
    ///  
    fn protocol_reset(&mut self)
    ///
    /// View_timeout_handler contains what to do when a view times out. 
    ///
    fn view_timeout_handler(&mut self,current_peer_id: PeerId) -> PhaseState
    ///
    /// It serializes the consensus data structure and generate a map fron phase_num(u8) to serialized data.
    ///  
    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>>
    ///
    /// It stores the map from phase_num(u8) to phase_name(String).
    ///
    fn phase_map(&self) -> HashMap<u8,String>
    ///
    /// It gets the current request the consensus protocol round is processing.
    ///
    fn current_request(&self) -> Request
    ///
    /// It checks whether this node is the leader.
    ///
    fn is_leader(&self, current_peer_id: Vec<u8>) -> bool
    ///
    /// It serializes the request data structure.
    ///
    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8>
    ///
    /// It checks whether the given request has been taken to the consensus process.
    ///
    fn check_taken_request(&self,request:Vec<u8>) -> bool
    ///
    /// It sets the request.
    ///
    fn set_current_request(&mut self, request: &Request)
```
### 2. Create the ```main``` Function
After the protocol customization process is complete, create the main function to run.
For example, the following is the PBFT protocol:

``` dart
    #[tokio::main]
    async fn main() -> Result<(), Box<dyn Error>> {
        let mut framework: BFTDiagnosisFramework<_> = BFTDiagnosisFramework::new();

        let consensus_node: ProtocolActuator<PBFTProtocol> = ProtocolActuator::new();
        framework.set_consensus_node(consensus_node);

        framework.run().await?;
        Ok(())
    }
```

### 3. Set the Configuration File
Users needs to set the protocol type, test items and testing parameters through the configuration file in ```BFTDiagnosis/src/config_files```.

All configuration parameters are organized in the format of .toml files.

After protocol customization and configration, users can start the test. There follows the steps.
### 4. Start Consensus Node
First, Users need to provide the specified IP, port and a bool value (is or not the leader) to start the consensus node, as follows

``` cargo run -- --consensus 10.176.34.71 6666 true```
### 5. Start Controller
Then, Users need to start the controller. There follows the steps.

``` cargo run -- --controller ```
### 6. Start Analyzer
Then, Users need to start the Analyzer. There follows the steps.

``` cargo run -- --analyzer ```
### 7. Init
Controller and Analyzer each perform initialization operations.

``` BFTDiagnosis(Controller) >> init ```

``` BFTDiagnosis(Analyzer)   >> init ```
### 8. Controller set the TestItem 
Controller configures the test items and transmits them to Analyzer.

``` BFTDiagnosis(Controller)>> configureAnalyzer ``` 
### 9. Controller set the state of Consensus Nodes
Controller set the state of Consensus Nodes by sending messages to Consensus Nodes.

``` BFTDiagnosis(Controller)>> configureConsensusNode  ```
### 10. Protocol Running
Controller sends commands to Consensus Nodes to initiate the process of the consensus protocol.

``` BFTDiagnosis(Controller)>> protocolStart ```
### 11. Start testing
Controller sends a command to Analyzer to initiate the process of the test.

``` BFTDiagnosis(Controller)>> startTest ```
### 12. View the results
Test results are stored in both local memory and Mysql-database, and can be viewed through both print and database queries.

``` BFTDiagnosis(Analyzer)>> printLatencyResults ```

``` mysql -> select * from LatencyResults ```