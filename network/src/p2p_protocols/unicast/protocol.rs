use std::{error, fmt, io, iter, pin::Pin};

use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, Future};
use libp2p::{
    core::{upgrade, UpgradeInfo},
    InboundUpgrade, OutboundUpgrade,
};
use serde::{Deserialize, Serialize};
use utils::coder;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnicastProtocol;

impl UpgradeInfo for UnicastProtocol {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/unicast/1.0.0")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for UnicastProtocol
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = UnicastMessage;
    type Error = UnicastDecodeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            let packet = upgrade::read_length_prefixed(&mut socket, 2048).await?;
            // println!("ppppppp");
            let unicast_msg: UnicastMessage = coder::deserialize_for_bytes(&packet[..]);
            //let unicast_msg: UnicastMessage = encode::deserialize_for_bytes(&packet[..]);
            // println!("iiiiiii");

            Ok(unicast_msg)
        })
    }
}

#[derive(Debug, Hash, Serialize, Deserialize)]
pub struct UnicastMessage {
    pub source: Vec<u8>,
    pub data: Vec<u8>,
    pub sequence_number: Vec<u8>,
}

impl UpgradeInfo for UnicastMessage {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/unicast/1.0.0")
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for UnicastMessage
where
    TSocket: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            let message_bytes = coder::serialize_into_bytes(&self);
            // println!("Message bytes: {}", &message_bytes.len());
            upgrade::write_length_prefixed(&mut socket, message_bytes).await?;
            socket.close().await?;

            Ok(())
        })
    }
}

#[derive(Debug)]
pub enum UnicastDecodeError {
    ReadError(io::Error),
    InvalidPeerId,
}

impl From<io::Error> for UnicastDecodeError {
    fn from(err: io::Error) -> Self {
        UnicastDecodeError::ReadError(err)
    }
}

impl fmt::Display for UnicastDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UnicastDecodeError::ReadError(ref err) => {
                write!(f, "Error while reading form socket: {}", err)
            }
            UnicastDecodeError::InvalidPeerId => {
                write!(f, "Error while decoding PeerId from message")
            }
        }
    }
}

impl error::Error for UnicastDecodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            UnicastDecodeError::ReadError(ref err) => Some(err),
            UnicastDecodeError::InvalidPeerId => None,
        }
    }
}
