use crate::blockchain::{State, Blockchain};
use crate::network::server::Handle as ServerHandle;
use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use crate::types::transaction::SignedTransaction;
use std::collections::HashMap;
use super::super::network::message::Message;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Worker {
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
}

impl Worker {
    pub fn new(
        blockchain: &Arc<Mutex<Blockchain>>,
        mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
    ) -> Self {
        let blockchain = Arc::clone(blockchain);
        let mempool = Arc::clone(mempool);
        Self {
            blockchain: blockchain,
            mempool: mempool,
            server: server.clone(),
            finished_block_chan
        }
    }
    pub fn validate_mempool(&self, mempool: &HashMap<H256, SignedTransaction>, curr_state: State) -> Vec<H256>{
        let accounts = curr_state.get_accounts();
        let mut tx_delete = vec![];
        for (hash, transaction) in mempool.iter() {
            let tx_sender = transaction.t.sender;
            let tx_sender_nonce = transaction.t.nonce;
            let tx_value = transaction.t.value;
            if accounts.contains_key(&tx_sender){
                let (state_account_nonce, state_account_bal) = accounts.get(&tx_sender).unwrap();
                if state_account_nonce >= &tx_sender_nonce || state_account_bal < &tx_value {
                    tx_delete.push(hash.clone());
                }
            }
        }
        tx_delete        
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

            // update the state and the chain here
            let mut b = self.blockchain.lock().unwrap();
            let mut m = self.mempool.lock().unwrap();
            if b.contains(&_block.get_parent()){
                if b.verify_block(&_block) && b.block_state.contains_key(&_block.get_parent()){
                    b.update_state(&_block);
                    b.insert(&_block);
                    println!("Tip State: {:?}", b.get_tip_state());
                    let curr_state = b.get_tip_state();
                    // need to validate the mempool and update the state
                    let tx_delete = self.validate_mempool(&m, curr_state);
                    for tx_hash in tx_delete {
                        m.remove(&tx_hash);
                    }
               
                    self.server.broadcast(Message::NewBlockHashes(vec![_block.hash()]));
                }
            }
            drop(b);
            drop(m);
        }
    }
}
