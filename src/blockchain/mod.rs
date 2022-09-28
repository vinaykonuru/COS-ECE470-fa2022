use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::time::SystemTime;

pub struct Blockchain {
    chain: HashMap<H256, Block>,
    head: H256,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut rng = thread_rng();
        // random nonce(doesn't have to solve the puzzle for the genesis according to Office Hours)
        let nonce: u32 = rng.gen();

        // random parent(okay according to Office Hours)
        let mut parent: H256 = [0; 32].into();
        parent = parent.hash();

        // empty root as well, genesis stores no data right now
        let mut merkle_root: H256 = [0; 32].into();
        merkle_root = merkle_root.hash();
        // arbitrary difficulty
        let difficulty: H256 = [5; 32].into();
        // current timestamp
        let timestamp = SystemTime::now();
        let mut chain: HashMap<H256, Block> = HashMap::new();
        let genesis = Block::new(
            parent,
            nonce,
            SystemTime::now(),
            difficulty,
            merkle_root,
            vec![],
        );
        let genesis_serial: Vec<u8> = bincode::serialize(&genesis).unwrap();
        let genesis_hash: H256 =
            ring::digest::digest(&ring::digest::SHA256, &genesis_serial).into();
        chain.insert(genesis_hash, genesis);
        Self {
            chain: HashMap::new(),
            head: genesis_hash,
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        unimplemented!()
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        unimplemented!()
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        // unimplemented!()
        vec![]
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
