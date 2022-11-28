pub mod worker;

use log::info;

use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::types::hash::{Hashable, H256};
use crate::types::merkle::MerkleTree;
use crate::types::transaction::SignedTransaction;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use rand::{thread_rng, Rng};
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};
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
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_block_chan: Sender<Block>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>) -> (Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();
    let blockchain = Arc::clone(blockchain);
    let mempool = Arc::clone(mempool);
    let ctx = Context {
        blockchain: blockchain,
        mempool: mempool,
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
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

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
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
        let mut rng = thread_rng();
        let mut timestamp: u128;
        let mut data: Vec<H256>;
        let mut content: Vec<SignedTransaction>;
        let mut parent: H256 = self.blockchain.lock().unwrap().tip();
        let mut height: usize = self.blockchain.lock().unwrap().get_tip_height();
        let mut merkle_root: H256;
        let mut difficulty: H256 = self
            .blockchain
            .lock()
            .unwrap()
            .get_block(&parent)
            .unwrap()
            .get_difficulty();

        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
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

            // build a block
            timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let mut content: Vec<SignedTransaction> = vec![];
            let mut data = vec![];
            let mut tx_tracker = HashSet::new();
            println!("in miner");
            let mut b = self.blockchain.lock().unwrap();
            println!("after blockchain, before mempool");
            let mut m = self.mempool.lock().unwrap();
            println!("after mempool");
            for (hash, transaction) in m.iter(){
                if content.len() == 3 {
                    break;
                }
                let mut pub_key = &transaction.pub_key;
                let mut state = b.get_tip_state();
                if transaction.verify(&state) {
                    if !tx_tracker.contains(&pub_key.clone()) {
                        tx_tracker.insert(pub_key.clone());
                        content.push(transaction.clone());
                        data.push(hash.clone());
                    }
                }
            }
            drop(b);
            drop(m);
            merkle_root = MerkleTree::new(&data).root();

            let nonce: u32 = rng.gen();

            let block: Block =
                Block::new(parent, nonce, timestamp, difficulty, merkle_root, content.clone());
            if block.hash() <= difficulty && !data.is_empty(){
                self.finished_block_chan.send(block.clone()).unwrap(); // this will handle placing it into the blockchain
                for tx_hash in data{
                    self.mempool.lock().unwrap().remove(&tx_hash);
                    // println!("Size of mempool after removal: {:?}", m.keys().len());
                }
                parent = block.hash();
            }

            // println!("Size of mempool: {:?}", m.keys().len());

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
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

    #[test]
    #[timeout(60000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        miner_ctx.start();
        miner_handle.start(0);
        let mut block_prev = finished_block_chan.recv().unwrap();
        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
