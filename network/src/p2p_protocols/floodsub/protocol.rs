use std::{error, fmt, io, iter, pin::Pin};

use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, Future};
use libp2p::{
    core::{upgrade, UpgradeInfo},
    InboundUpgrade, OutboundUpgrade, PeerId,
};
use serde::{Deserialize, Serialize};
use utils::coder;

use super::topic::Topic;

/// Implementation of `ConnectionUpgrade` for the floodsub protocol.
#[derive(Debug, Clone, Default)]
pub struct FloodsubProtocol {}

impl FloodsubProtocol {
    /// Builds a new `FloodsubProtocol`.
    pub fn new() -> FloodsubProtocol {
        FloodsubProtocol {}
    }
}

impl UpgradeInfo for FloodsubProtocol {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/floodsub/1.0.0")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for FloodsubProtocol
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = FloodsubData;
    type Error = FloodsubDecodeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            let packet = upgrade::read_length_prefixed(&mut socket, 30000).await?;
            // println!("***********************************************\n{:?}",packet.clone());
            let floodsub_msg: FloodsubData = coder::deserialize_for_bytes(&packet[..]);

            let mut messages = Vec::with_capacity(floodsub_msg.messages.len());
            
            for message in floodsub_msg.clone().messages {
                messages.push(FloodsubMessage {
                    source: message.source,
                    data: message.data,
                    sequence_number: message.sequence_number,
                    topics: message.topics,
                });
            }

            Ok(FloodsubData {
                messages,
                subscriptions: floodsub_msg
                    .subscriptions
                    
            })
            
        })
    }
}

/// An RPC received by the floodsub system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FloodsubData {
    pub messages: Vec<FloodsubMessage>,
    pub subscriptions: Vec<FloodsubSubscription>,
}

impl UpgradeInfo for FloodsubData {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/floodsub/1.0.0")
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for FloodsubData
where
    TSocket: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            // let bytes = self.into_bytes();
            let bytes = coder::serialize_into_bytes(&self);
            // println!("Test:{:?}",bytes.clone());
            upgrade::write_length_prefixed(&mut socket, bytes).await?;
            socket.close().await?;

            Ok(())
        })
    }
}

/// A message received by the floodsub system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FloodsubMessage {
    /// Id of the peer that published this message.
    pub source: Vec<u8>,

    /// Content of the message. Its meaning is out of scope of this library.
    pub data: Vec<u8>,

    /// An incrementing sequence number.
    pub sequence_number: Vec<u8>,

    /// List of topics this message belongs to.
    ///
    /// Each message can belong to multiple topics at once.
    pub topics: Vec<Topic>,
}

/// A subscription received by the floodsub system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FloodsubSubscription {
    /// Action to perform.
    pub action: FloodsubSubscriptionAction,
    /// The topic from which to subscribe or unsubscribe.
    pub topic: Topic,
}

/// Action that a subscription wants to perform.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize )]
pub enum FloodsubSubscriptionAction {
    /// The remote wants to subscribe to the given topic.
    Subscribe,
    /// The remote wants to unsubscribe from the given topic.
    Unsubscribe,
}

#[derive(Debug)]
pub enum FloodsubDecodeError {
    ReadError(io::Error),
    InvalidPeerId,
}

impl From<io::Error> for FloodsubDecodeError {
    fn from(err: io::Error) -> Self {
        FloodsubDecodeError::ReadError(err)
    }
}

impl fmt::Display for FloodsubDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FloodsubDecodeError::ReadError(ref err) => {
                write!(f, "Error while reading form socket: {}", err)
            }
            FloodsubDecodeError::InvalidPeerId => {
                write!(f, "Error while decoding PeerId from message")
            }
        }
    }
}

impl error::Error for FloodsubDecodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FloodsubDecodeError::ReadError(ref err) => Some(err),
            FloodsubDecodeError::InvalidPeerId => None,
        }
    }
}
