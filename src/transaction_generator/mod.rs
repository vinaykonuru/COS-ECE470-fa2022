use log::info;
use crate::blockchain::{State, Blockchain};
use crate::types::block::Block;
use crate::types::hash::{Hashable, H256};
use crate::types::merkle::MerkleTree;
use crate::types::transaction;
use crate::types::address::Address;
use crate::types::transaction::SignedTransaction;
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::types::key_pair;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use rand::{thread_rng, Rng};
use ring::signature::{KeyPair, Ed25519KeyPair};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update,     // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    server: ServerHandle,
    key_pair: Ed25519KeyPair,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<HashMap<H256,SignedTransaction>>>, server: &ServerHandle, key_pair: Ed25519KeyPair) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let blockchain = Arc::clone(blockchain);
    let mempool = Arc::clone(mempool);
    let ctx = Context {
        blockchain: blockchain,
        mempool: mempool,
        server: server.clone(),
        key_pair: key_pair,
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
    };
    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

#[cfg(any(test, test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) {
    let blockchain = Blockchain::new();
    new(&Arc::new(Mutex::new(blockchain)))
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, theta: u64) {
        self.control_chan
            .send(ControlSignal::Start(theta))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Transaction generator shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Transaction generator starting in continuous mode with theta {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            let mut b = self.blockchain.lock().unwrap();
            let mut m = self.mempool.lock().unwrap();
            let mut rng = thread_rng();
            let public_keys : Vec<Address> = b.get_tip_state().get_accounts().keys().cloned().collect();
            let sender_key_pair = &self.key_pair;
            let receiver_addr = &public_keys[rng.gen_range(0..=2)];
            drop(b);
            drop(m);

            let state = 
            {let mut b = self.blockchain.lock().unwrap();
                b.get_tip_state()
            };
            let sender_addr = Address::from_public_key_bytes(sender_key_pair.public_key().as_ref());
            let (sender_nonce, sender_bal) = state.get_accounts().get(&sender_addr).unwrap().clone();
            
            if sender_bal > 1 {
                let random_transaction = transaction::generate_signed_transaction(sender_key_pair, &receiver_addr, &sender_nonce, &sender_bal);
                {let mut m = self.mempool.lock().unwrap();
                    println!("inserting to mempool: {:?}", m.len());
                    m.insert(random_transaction.hash(), random_transaction.clone());
                }
                

                self.server.broadcast(Message::NewTransactionHashes(vec![random_transaction.hash()]))
            }
            
            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval * 10000);                    
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use crate::types::hash::Hashable;
    use ntest::timeout;

}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
