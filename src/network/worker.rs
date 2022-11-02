use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use crate::blockchain::Blockchain;
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
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
    ) -> Self {
        Self {
            blockchain: Arc::clone(blockchain),
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
            let mut blockchain = self.blockchain.lock().unwrap();
            println!("{:?}", blockchain.get_tip_height());
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
                    let mut new_hashes : Vec<H256> = Vec::new();
                    for hash in hashes{
                        // if blockchain doesn't contain a hash, add it to new hashes
                        if !blockchain.contains(&hash){
                            new_hashes.push(hash);
                        }
                    }
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
                        match blockchain.get_block(&hash) {
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
                    for block in blocks{
                        let hash : H256 = block.hash();
                        // PoW Validity Check:
                        if !blockchain.contains(&block.get_parent()){
                            // add block to buffer
                            buffer.add(block.clone(), block.get_parent());
                            // broadcast that we're missing a block's parent
                            peer.write(Message::GetBlocks(vec![hash]));
                            self.server.broadcast(Message::GetBlocks(vec![hash]));
                        }
                        else{
                            let parent : Block = blockchain.get_block(&block.get_parent()).unwrap();
                            if block.hash() <= block.get_difficulty() && block.get_difficulty() == parent.get_difficulty(){
                                if !blockchain.contains(&parent.hash()){

                                }
                                else{
                                    // add block to chain
                                    if !blockchain.contains(&hash){
                                        blockchain.insert(&block);
                                        new_blocks.push(block.clone());
                                    }
                                }
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
                                        blockchain.insert(&temp_block);
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