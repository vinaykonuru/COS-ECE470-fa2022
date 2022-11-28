use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use crate::types::transaction::SignedTransaction;
use crate::blockchain::{State, Blockchain};
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::{debug, warn, error};

use std::thread;

#[cfg(any(test,test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test,test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;
#[derive(Clone)]
pub struct Worker {
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
}

pub struct OrphanBuffer {
    // key is parent hash value is block
    buffer : HashMap<H256, Vec<Block>>
}

impl OrphanBuffer {
    pub fn new() -> Self {        
        let mut buffer: HashMap<H256, Vec<Block>> = HashMap::new();
        Self{buffer}
    }
    pub fn add(&mut self, block : Block, parent_hash: H256) {
        if self.contains(parent_hash){
            let mut arr : Vec<Block> = self.buffer.get(&parent_hash).unwrap().clone();
            arr.push(block);
            self.buffer.insert(parent_hash, arr.clone());
        }
        else{
            self.buffer.insert(parent_hash, vec![block]);
        }
    }
    pub fn contains(&self, parent_hash:H256) -> bool {
        self.buffer.contains_key(&parent_hash)
    }
    pub fn get(&mut self, parent_hash:H256) -> Vec<Block> {
        let mut temp_vec = vec![];
        if self.contains(parent_hash){
            temp_vec = self.buffer.remove(&parent_hash).unwrap().clone()
        }
        temp_vec
    }
}
impl Worker {
    pub fn new(
        blockchain: &Arc<Mutex<Blockchain>>,
        mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
    ) -> Self {
        Self {
            blockchain: Arc::clone(blockchain),
            mempool: Arc::clone(mempool),
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }
    pub fn validate_mempool(&self, tx_set: Vec<SignedTransaction>, curr_state: State) -> Vec<H256>{
        let accounts = curr_state.get_accounts();
        let mut tx_delete = vec![];
        for transaction in tx_set {
            let tx_sender = transaction.t.sender;
            let tx_sender_nonce = transaction.t.nonce;
            if accounts.contains_key(&tx_sender){
                let state_account_nonce = accounts.get(&tx_sender).unwrap().0;
                if state_account_nonce <= tx_sender_nonce {
                    tx_delete.push(transaction.hash().clone());
                }
            }
        }
        tx_delete        
    }
    fn worker_loop(&self) {
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            let mut buffer = OrphanBuffer::new();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hashes) => {
                    // if hashes are not in blockchain, send the following:
                    println!("new block hashes");
                    let mut new_hashes : Vec<H256> = Vec::new();
                    for hash in hashes{
                        // if blockchain doesn't contain a hash, add it to new hashes
                        if !self.blockchain.lock().unwrap().contains(&hash){
                            new_hashes.push(hash);
                        }
                    }
                    println!("looped through hashes in new blocks");
                    // ask for hashes the local miner doesn't have
                    if new_hashes.len() != 0 {
                        peer.write(Message::GetBlocks(new_hashes.clone()));
                        self.server.broadcast(Message::GetBlocks(new_hashes));
                    }
                }
                Message::GetBlocks(hashes) => {
                    // if hashes are in blockchain, get blocks and send out a message with them
                    let mut blocks: Vec<Block> = Vec::new();
                    for hash in hashes{
                        match self.blockchain.lock().unwrap().get_block(&hash) {
                            Some(block) => blocks.push(block),
                            _ => {}
                        }
                    }
                    // push the blocks it does have
                    peer.write(Message::Blocks(blocks.clone()));
                    self.server.broadcast(Message::Blocks(blocks));
                }
                Message::Blocks(blocks) => {
                    // add these blocks to blockchain if they're not already in it, noting the ones that are new
                    let mut new_blocks : Vec<Block> = Vec::new();
                    println!("entering blocks");
                    for block in blocks{
                        let hash : H256 = block.hash();
                        // PoW Validity Check
                        if block.hash() > block.get_difficulty(){
                            continue;
                        }
                        { let blockchain = self.blockchain.lock().unwrap();
                            if blockchain.contains(&hash){
                                continue;
                            }
                            println!("entering if statement");
                        };
                        // check if blockchain has block's parent, add to buffer if it doesn't
                        println!("blocks message 3");
                        if !self.blockchain.lock().unwrap().contains(&block.get_parent()){
                            buffer.add(block.clone(), block.get_parent());
                            // broadcast that we're missing a block's parent
                            peer.write(Message::GetBlocks(vec![block.get_parent()]));
                            self.server.broadcast(Message::GetBlocks(vec![block.get_parent()]));
                        }
                        else{

                            // verify block's validity and add it to chain
                            {
                            println!("blocks message 4");
                            let mut blockchain = self.blockchain.lock().unwrap();
                            let mut mempool = self.mempool.lock().unwrap();
                            println!("blocks message 5");
                            if blockchain.verify_block(&block) && blockchain.block_state.contains_key(&block.get_parent()){
                                println!("receiving blocks");
                                let valid_block = blockchain.update_state(&block);
                                if !valid_block {
                                    continue;
                                }
                                blockchain.insert(&block);
                                for transaction in block.get_content(){
                                    mempool.remove(&transaction.hash());
                                }
                                let state = blockchain.get_state(&block);
                                println!("State After Insertion: {:?}",state);
                                let tx_set : Vec<SignedTransaction> = mempool.values().cloned().collect();
                                let tx_delete = self.validate_mempool(tx_set, state);
                                for tx_hash in tx_delete{
                                    mempool.remove(&tx_hash);
                                }
                                new_blocks.push(block.clone());
                                // do this iteratively
                                // check if the block is a parent in the buffer, iteratively add all blocks
                                if buffer.contains(hash){
                                    let mut orphans = VecDeque::new();
                                    let mut temp_orphans = buffer.get(hash);
                                    for block in temp_orphans{
                                        orphans.push_back(block);
                                    }
                                    while !orphans.is_empty(){
                                        let temp_block = orphans.pop_front().unwrap();
                                        if blockchain.verify_block(&temp_block){
                                            blockchain.update_state(&temp_block);
                                            blockchain.insert(&temp_block);
                                            // need to validate the mempool everytime we update the state
                                            let tx_set : Vec<SignedTransaction> = mempool.values().cloned().collect();
                                            let tx_delete = self.validate_mempool(tx_set, blockchain.get_state(&block));
                                            for tx_hash in tx_delete {
                                                mempool.remove(&tx_hash);
                                            }
                                        }
                                        for transaction in block.get_content(){
                                            mempool.remove(&transaction.hash());
                                            
                                        }
                                        new_blocks.push(temp_block.clone());
                                        if buffer.contains(temp_block.hash()){
                                            temp_orphans = buffer.get(block.hash());
                                            for block in temp_orphans{
                                                orphans.push_back(block.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            };
                        }
                    }
 
                    // then get the hashes of the blocks that are new
                    let mut new_hashes : Vec<H256> = Vec::new();
                    for block in new_blocks {
                        new_hashes.push(block.hash());
                    }
                    // and broadcast these new hash blocks
                    peer.write(Message::NewBlockHashes(new_hashes.clone()));
                    self.server.broadcast(Message::NewBlockHashes(new_hashes));
                }
                Message::NewTransactionHashes(hashes) => {
                    let mut new_hashes : Vec<H256> = Vec::new();
                    for hash in hashes{
                        // if blockchain doesn't contain a hash, add it to new hashes
                        {let m = self.mempool.lock().unwrap();
                            if !m.contains_key(&hash){
                                new_hashes.push(hash);
                            }
                        };
                    }
                    // ask for hashes the local miner doesn't have
                    if new_hashes.len() != 0 {
                        peer.write(Message::GetTransactions(new_hashes.clone()));
                        self.server.broadcast(Message::GetTransactions(new_hashes));
                    }
                }
                Message::GetTransactions(hashes) => {
                    // if hashes are in blockchain, get blocks and send out a message with them
                    let mut transactions: Vec<SignedTransaction> = Vec::new();
                    for hash in hashes{
                        {let m = self.mempool.lock().unwrap();
                            match m.get(&hash) {
                                Some(transaction) => transactions.push(transaction.clone()),
                                _ => {}
                            }
                        };
                    }
                    // push the blocks it does have
                    peer.write(Message::Transactions(transactions.clone()));
                    self.server.broadcast(Message::Transactions(transactions));
                }
                Message::Transactions(transactions) => {
                    // add these blocks to blockchain if they're not already in it, noting the ones that are new
                    let mut new_transactions : Vec<SignedTransaction> = Vec::new();
                    let mut b = self.blockchain.lock().unwrap();
                    let mut m = self.mempool.lock().unwrap();
                    for transaction in transactions{
                        let hash : H256 = transaction.hash();
                        let curr_state = b.get_tip_state();
                        if transaction.verify(&curr_state){
                            m.insert(transaction.hash(), transaction.clone());
                            new_transactions.push(transaction.clone());
                        }
                    }
                    // then get the hashes of the blocks that are new
                    let mut new_hashes : Vec<H256> = Vec::new();
                    for transaction in new_transactions {
                        new_hashes.push(transaction.hash());
                    }
                    drop(b);
                    drop(m);

                    // and broadcast these new hash blocks
                    peer.write(Message::NewBlockHashes(new_hashes.clone()));
                    self.server.broadcast(Message::NewBlockHashes(new_hashes));
                }
                _ =>{}
            }
        }
    }
}

#[cfg(any(test,test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>
}
#[cfg(any(test,test_utilities))]
impl TestMsgSender {
    fn new() -> (TestMsgSender, smol::channel::Receiver<(Vec<u8>, peer::Handle)>) {
        let (s,r) = smol::channel::unbounded();
        (TestMsgSender {s}, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let worker = Worker::new(&blockchain, 1, msg_chan, &server);
    worker.start();
    let mut hashes : Vec<H256> = vec![];
    let mut curr = Some(blockchain.lock().unwrap().head());

    while let Some(x) = curr {
        hashes.push(x.clone().hash());
        curr = blockchain.lock().unwrap().get_block(&x.get_parent());
    }
    (test_msg_sender, server_receiver, hashes)
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(1000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();
        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST