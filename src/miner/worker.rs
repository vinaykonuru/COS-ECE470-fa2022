use crate::blockchain::Blockchain;
use crate::network::server::Handle as ServerHandle;
use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use super::super::network::message::Message;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Worker {
    blockchain: Arc<Mutex<Blockchain>>,
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
}

impl Worker {
    pub fn new(
        blockchain: &Arc<Mutex<Blockchain>>,
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
    ) -> Self {
        let blockchain = Arc::clone(blockchain);
        Self {
            blockchain: blockchain,
            server: server.clone(),
            finished_block_chan,
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _block = self
                .finished_block_chan
                .recv()
                .expect("Receive finished block error");
            // TODO for student: insert this finished block to blockchain, and broadcast this block hash
            self.blockchain.lock().unwrap().insert(&_block);
            // broadcasting in another assignment
            self.server.broadcast(Message::NewBlockHashes(vec![_block.hash()]))
        }
    }
}
