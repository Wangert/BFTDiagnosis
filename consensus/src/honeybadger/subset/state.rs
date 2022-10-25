use crate::honeybadger::binary_agreement::binary_agreement::BinaryAgreement;
use crate::honeybadger::binary_agreement::AbaMessage as AbaMessageContent;
use crate::honeybadger::binary_agreement::Message as AbaMessage;
use crate::honeybadger::reliable_broadcast::broadcast::Broadcast;
use crate::honeybadger::reliable_broadcast::message::Message as RbcMessage;
use crate::honeybadger::{network::Network, reliable_broadcast::broadcast::PeerSet};
use crate::pbft::state;
use libp2p::tcp::async_io;
use std::collections::HashSet;
use std::mem;
use tokio::sync::mpsc::{self, Receiver, Sender};
use utils::coder;
#[derive(Debug)]
pub enum State {
    //等待可靠广播的结果和二进制共识的结果
    Ongoing(Broadcast, BinaryAgreement),
    //收到了广播的结果，等待二进制共识的结果
    GetValue(Vec<u8>, BinaryAgreement),
    //已经收到二进制公式的结果，等待可靠广播的结果（RBC未完成）
    Accepted(Broadcast),
    //该节点的ACS已经完成，true表示该节点的value将被输出，false表示不被输出
    Complete(bool),
}

impl State {
    //创建一个新的Ongoing状态的state
    pub fn new(
        netinfo: Network,
        nonce: usize,
        rbc_msg_tx: Sender<Vec<u8>>,
        aba_msg_tx: Sender<Vec<u8>>,
        proposer_id: String,
    ) -> Self {
        let mut broadcast = Broadcast::new(netinfo.id.as_str(), netinfo.clone());
        broadcast.msg_tx = rbc_msg_tx;
        let mut binary_agreement = BinaryAgreement::new(netinfo, nonce, proposer_id.as_str());
        binary_agreement.ba_broadcast.msg_sender = aba_msg_tx;

        State::Ongoing(broadcast, binary_agreement)
    }

    /// 是否收到了可靠广播的结果，RBC阶段完成
    pub fn received(&self) -> bool {
        match self {
            State::Ongoing(_, _) | State::Accepted(_) => false,
            State::GetValue(_, _) => true,
            State::Complete(accepted) => *accepted,
        }
    }

    /// 提议是否被接受，不管是否收到了该消息,ABA阶段完成
    pub fn accepted(&self) -> bool {
        match self {
            State::Ongoing(_, _) | State::GetValue(_, _) => false,
            State::Accepted(_) => true,
            State::Complete(accepted) => *accepted,
        }
    }

    // 提议是否被拒绝/接受
    pub fn complete(&self) -> bool {
        match self {
            State::Complete(_) => true,
            _ => false,
        }
    }

    //提出一个提案
    pub async fn propose(
        &mut self,
        val: Vec<u8>,
        netinfo: crate::honeybadger::network::Network,
        source: &str,
    ) {
        println!("开始RBC的propose阶段*************************");
        //self.transition(|state| state.handle_broadcast(|bc| bc.broadcast(value)))
        match self {
            State::Ongoing(rbc, aba) => {
                for (id, peer) in netinfo.connected_nodes {
                    if (peer.as_str() != source) {
                        rbc.peer_set.insert(peer.clone());
                        rbc.ready_sent.entry(peer.clone()).or_insert(false);
                        rbc.echo_set.entry(peer).or_insert(HashSet::new());
                    } else {
                    }
                }
                rbc.handle_input(val).await;
                println!("RBC propose已完成，VAL广播完成！");
                //aba.propose(true).await;
            }
            _ => {}
        }
    }

    pub fn vote_false(&mut self) {
        //self.transition(|state| state.handle_agreement(|ba| ba.propose(false).await))
    }

    // pub  fn handle_message(&mut self,sender_id: &str,msg :super::message::Message) {
    //     match msg {
    //         super::message::Message::Broadcast(proposer_id,rbc_msg) => {
    //             let msg = coder::serialize_into_bytes(&rbc_msg);
    //             //self.handle_broadcast( |rbc| async {rbc.handle_message(sender_id, &msg).await});

    //         },
    //         super::message::Message::Agreement(proposer_id,aba_msg) => {
    //             let msg = coder::serialize_into_bytes(&aba_msg);
    //             self.handle_agreement(|aba| aba.handle_message(sender_id, &msg))
    //         },
    //         _ => ()
    //     }
    // }

    pub async fn handle_message(&mut self, sender_id: &str, msg: super::message::MessageContent)  {
        let new_state = mem::replace(self, State::Complete(false));
        match msg {
            super::message::MessageContent::Broadcast(proposer_id, rbc_msg) => {
                let transition = new_state.handle_broadcast(sender_id, rbc_msg).await;
                *self = transition;
                println!("RBC消息处理完毕，状态切换完成！");
                
            }
            super::message::MessageContent::Agreement(proposer_id, aba_msg) => {
                let transition = new_state.handle_agreement(sender_id, aba_msg).await;
                *self = transition;
                println!("ABA消息处理完毕，状态切换完成！");
            }
            _ => todo!(),
        }
    }

    async fn handle_broadcast(self, sender_id: &str, msg: RbcMessage) -> Self {
        let data = coder::serialize_into_bytes(&msg);
        match self {
            State::Ongoing(mut rbc, mut aba) => match rbc.handle_message(sender_id, &data).await {
                Some(value) => {
                    println!("已收到RBC消息，开始进行ABA的propose！");
                    aba.propose(true).await;
                    println!("ABA propose完成！");
                    State::GetValue(value, aba)
                }
                None => {
                    println!("还未收到RBC消息，继续执行RBC");
                    State::Ongoing(rbc, aba)
                }
            },
            State::Accepted(mut rbc) => match rbc.get_output() {
                Some(value) => State::Complete(true),
                None => State::Accepted(rbc),
            },
            state @ State::GetValue(_, _) | state @ State::Complete(_) => state,
        }
    }

    async fn handle_agreement(self, sender_id: &str, msg: AbaMessage) -> Self {
        let data = coder::serialize_into_bytes(&msg);
        match self {
            State::Ongoing(rbc, mut aba) => match aba.handle_message(sender_id, &data).await {
                Some(true) => State::Accepted(rbc),
                Some(false) => State::Complete(false),
                None => State::Ongoing(rbc, aba),
            },
            State::GetValue(value, mut aba) => match aba.handle_message(sender_id, &data).await {
                Some(true) => State::Complete(true),
                Some(false) => State::Complete(false),
                None => State::GetValue(value, aba),
            },
            state @ State::Accepted(_) | state @ State::Complete(_) => state,
        }
    }

    
}
