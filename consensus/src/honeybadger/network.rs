use std::{collections::HashMap, sync::Arc};

use threshold_crypto::{
    PublicKey, PublicKeySet, PublicKeyShare, SecretKeySet, SecretKeyShare, Signature,
    SignatureShare,
};

#[derive(Debug, Clone)]
pub struct Network {
    //节点的ID
    pub id: String,
    pub connected_nodes: HashMap<usize, String>,
    peer_num: usize,
    fault_num: usize,
    //判断是否是共识节点
    is_consensus_node: bool,
    //节点的私钥share
    secret_key_share: Option<SecretKeyShare>,
    public_key_share: Option<PublicKeyShare>,
    //其他节点的PublicKeyShare
    public_key_shares: HashMap<String, PublicKeyShare>,
    public_key_set: Option<PublicKeySet>,
    //node_set: Arc<NodeSet>
}

impl Network {
    pub fn new(
        id: &str,
        is_consensus_node: bool,
        sks: Option<SecretKeyShare>,
        pk:Option<PublicKeyShare>,
        pks: Option<PublicKeySet>,
        connected_peers: Vec<String>,
    ) -> Self {
        //let public_key_shares: HashMap<String,PublicKeyShare> = HashMap::new();
        // let connected_nodes: HashMap<usize, String> = connected_peers
        //     .iter()
        //     .map(|p| {
        //         let peer = connected_peers.pop().unwrap();
        //         (num - connected_peers.len(), peer)
        //     })
        //     .collect();
        let mut connected_nodes = HashMap::new();
        for (index,value) in connected_peers.iter().enumerate(){
            connected_nodes.entry(index).or_insert(value.to_string());
        }

        let mut public_key_shares = HashMap::new();
        for (id,s) in connected_nodes.iter().enumerate(){
            public_key_shares.entry(connected_nodes.get(&id).unwrap().to_string()).or_insert(pks.clone().unwrap().public_key_share(id));
        }
        // let public_key_shares:HashMap<String, PublicKeyShare> = connected_nodes.iter().map(|p| {
        //     (*p.1,pks.public_key_share(p.0))
        // }).collect();

        Network {
            id: id.to_string(),
            peer_num: 4,
            fault_num: 1,
            is_consensus_node,
            secret_key_share: sks,
            public_key_share:pk,
            public_key_shares,
            public_key_set: pks,
            connected_nodes,
        }
    }

    pub fn get_public_key_set(&self) -> PublicKeySet{
        self.public_key_set.clone().unwrap()
    }

    pub fn get_fault_num(&self) -> usize {
        self.fault_num
    }

    pub fn num_honest(&self) -> usize {
        self.peer_num - self.fault_num
    }

    pub fn get_secret_key_share(&self) -> Option<SecretKeyShare> {
        Some(self.secret_key_share.clone().unwrap())
    }

    pub fn get_public_key_share(&self, id: &str) -> Option<&PublicKeyShare> {
        self.public_key_shares.get(id)
    }
}
