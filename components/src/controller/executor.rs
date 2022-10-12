use std::sync::Arc;

use storage::database::LevelDB;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Notify,
};
use utils::crypto::eddsa::{EdDSAKeyPair, EdDSAPublicKey};

// Controller node executor
pub struct Executor {
    // pub state: State,
    pub db: Box<LevelDB>,
    pub keypair: Box<EdDSAKeyPair>,
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
    pub timeout_notify: Arc<Notify>,
}

impl Executor {
    pub fn new(db_path: &str) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);

        Self {
            // state: State::new(),
            // log: Box::new(Log::new()),
            db: Box::new(LevelDB::new(db_path)),
            keypair: Box::new(EdDSAKeyPair::new()),
            msg_tx,
            msg_rx,
            timeout_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn message_handler(&mut self, _current_peer_id: &[u8], msg: &Vec<u8>) {
        // let _message: Message = coder::deserialize_for_bytes(msg);
        // match message.msg_type {

        // }
    }

    pub fn get_public_key_by_peer_id(&self, peer_id: &[u8]) -> Option<EdDSAPublicKey> {
        let value = self.db.read(peer_id);
        if let Some(pk_vec) = value {
            Some(EdDSAPublicKey(pk_vec))
        } else {
            None
        }
    }
}
