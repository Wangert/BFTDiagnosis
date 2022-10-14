pub mod ba_broadcast;
pub mod binary_agreement;
pub mod bool_set;

use std::collections::HashSet;

use serde::{Serialize,Deserialize};
use threshold_crypto::{
    PublicKey, PublicKeySet, PublicKeyShare, SecretKeySet, SecretKeyShare, Signature,
    SignatureShare,
};
use utils::coder::{serialize_into_bytes,deserialize_for_bytes,self};

use self::bool_set::BoolSet;

/// A threshold signing message, containing a signature share.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SigMessage(pub SignatureShare);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AbaMessage{
    BaBroadcast(String,ba_broadcast::Message),
    Conf(String,BoolSet),
    Term(String,bool),
    Coin(String,Box<SigMessage>),
    //SignatureShare(Box<SigMessage>),
    Signature(String,Box<Signature>)
}

impl AbaMessage{
    fn with_epoch(self,epoch: usize) -> Message{
        Message { epoch, content: self }
    }
}

/// Messages sent during the Binary Agreement stage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message {
    /// The `BinaryAgreement` epoch this message belongs to.
    pub epoch: usize,
    /// The message content for the `epoch`.
    pub content: AbaMessage,
}

