use structopt::StructOpt;


#[derive(StructOpt, Debug)]
pub struct Args {
    #[structopt(long = "maddr", default_value = "10.162.133.212")]
    pub mdns_addr: String,
    #[structopt(long = "mport", default_value = "51002")]
    pub mdns_port: u16,
    #[structopt(long = "gaddr", default_value = "10.162.133.212")]
    pub gossipsub_addr: String,
    #[structopt(long = "gport", default_value = "51102")]
    pub gossipsub_port: u16,
    #[structopt(long = "addr", default_value = "10.162.133.212")]
    pub swarm_addr: String,
    #[structopt(long = "port", default_value = "51102")]
    pub swarm_port: u16,
    #[structopt(long = "consensus", default_value = "false")]
    pub is_consensus_node: String,
}

