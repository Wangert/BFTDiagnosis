use super::binary_agreement::SigMessage;
use super::network::Network;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use threshold_crypto::{SignatureShare, Signature};

#[derive(Debug)]
pub struct ThresholdSign {
    //公式网络中的节点信息
    netinfo: Network,
    // 该节点收到的SignatureShare信息
    sigshare_received: HashMap<String, (usize,SignatureShare)>,
    // 本节点是否发送了Share
    had_input: bool,
    /// Termination flag.
    terminated: bool,
}

impl ThresholdSign {
    pub fn new(netinfo: Network) -> Self {
        ThresholdSign {
            netinfo,
            sigshare_received: HashMap::new(),
            had_input: false,
            terminated: false,
        }
    }

    //节点用自己的私钥share签名并广播
    pub fn sign(&mut self) -> Option<SigMessage> {
        if self.had_input {
            // Don't waste time on redundant shares.
            return None;
        }
        self.had_input = true;
        let msg: Option<SigMessage> = match self.netinfo.get_secret_key_share() {
            Some(sks) => Some(SigMessage(sks.sign("coin"))),
            None => None,
        };
        let id = self.netinfo.id.clone();
        self.handle_message(id.as_str(), &msg.clone().unwrap());
        msg

    }

    /// 处理来自source节点的包含signature share的消息
    ///在每次从其他节点收到消息时调用
    /// 如果收到f+1个节点的share，就可以合成完整的签名

    pub fn handle_message(&mut self, source: &str, message: &SigMessage)->Option<Signature> {
        if self.terminated {
            return None
        }
        //let SigMessage(share) = message;
        //检查收到的share是否有效
        if !self.check_share(source, message) {
            return None
        }
        let mut id:usize= 9999;
        for p in self.netinfo.connected_nodes.iter(){
            if p.1.as_str() == source{
                id = *p.0
            }else {
                
            }
        }
        if id != 9999{
            self.sigshare_received
            .insert(source.to_string(), (id,message.to_owned().0));
        }
        
        self.try_output()
    }

    pub fn check_share(&mut self, id: &str, msg: &SigMessage) -> bool {
        match self.netinfo.get_public_key_share(id) {
            Some(pk_i) => pk_i.verify(&msg.0, "coin"),
            None => false,
        }
    }

    pub fn try_output(&mut self) -> Option<Signature>{
        //收到f+1个signatureshare后合成为一个signature
        if !self.terminated && self.sigshare_received.len() > self.netinfo.get_fault_num() {
            let sig = self.combine_and_verify_sig();
            self.terminated = true;
            self.sign(); // Before terminating, make sure we sent our share.
            println!(" output {:?}", sig);
            sig
            //Ok(step.with_output(sig))
        } else {
            println!(
                "received {} shares, {}",
                self.sigshare_received.len(),
                if self.had_input { ", had input" } else { "" }
            );
            None
        }
    }

    fn combine_and_verify_sig(&self) -> Option<Signature> {
        
        let shares = self.sigshare_received
        .values()
        .map(|&(ref idx, ref share)| (idx, share));

        let sig = self
            .netinfo
            .get_public_key_set()
            .combine_signatures(shares)
            .expect("合成签名失败！");
        if !self
            .netinfo
            .get_public_key_set()
            .public_key()
            .verify(&sig, "coin")
        {
            None
        } else {
            Some(sig)
        }
    }
}
