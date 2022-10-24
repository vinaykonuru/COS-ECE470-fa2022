use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    chain: HashMap<H256, (Block, usize)>,
    head: Block,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        // random nonce(doesn't have to solve the puzzle for the genesis according to Office Hours)
        let nonce: u32 = 00000000000000000000000000000000;

        // random parent(okay according to Office Hours)
        let mut parent: H256 = [0; 32].into();
        // empty root as well, genesis stores no data right now
        let mut merkle_root: H256 = [0; 32].into();
        merkle_root = merkle_root.hash();
        // arbitrary difficulty
        let difficulty: H256 = [10; 32].into();
        // current timestamp
        let timestamp : u128 = 0;
        let height = 0;
        let mut chain: HashMap<H256, (Block, usize)> = HashMap::new();
        let genesis = Block::new(
            parent,
            nonce,
            timestamp,
            difficulty,
            merkle_root,
            vec![],
        );
        chain.insert(genesis.hash(), (genesis.clone(), height));
        println!("{:?}", genesis.hash());
        Self {
            chain: chain,
            head: genesis.clone(),
        }
    }
    pub fn contains(&self, key: &H256) -> bool{
        self.chain.contains_key(key)
    }
    pub fn get_block(&self, block_hash: &H256) -> Option<Block> {
        let block: Option<Block>;
        match self.chain.get(block_hash) {
            Some(res) => block = Some(res.0.clone()),
            None => block = None,
        }
        block
    }
    fn get_height(&self, block: &Block) -> usize {
        self.chain.get(&block.hash()).unwrap().1
    }
    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let tip_height: usize = self.get_tip_height();
        println!("{:?}", block.hash());
        let parent_height: usize = self.chain.get(&block.get_parent()).unwrap().1;
        let block_height: usize = parent_height + 1;
        
        self.chain
            .insert(block.hash(), (block.clone(), block_height));
        // rule = only make the fork the new longest chain if the fork tip is strictly longer than the current tip
        if self.tip() == block.get_parent() || block_height > tip_height {
            self.head = block.clone();
        }
    }
    pub fn head(&self) -> Block{
        self.head.clone()
    }
    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.head.hash()
    }
    pub fn get_tip_height(&self) -> usize {
        self.chain.get(&self.tip()).unwrap().1
    }
    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut curr_block: Block = self.head.clone();
        let mut parent_hash: H256;
        let mut count = 1;
        let longest_chain_len: usize = self.get_height(&self.head) + 1;
        let mut list: Vec<H256> = vec![curr_block.hash(); longest_chain_len];
        list[longest_chain_len - 1] = curr_block.hash();
        loop {
            parent_hash = curr_block.get_parent();
            list[longest_chain_len - 1 - count] = parent_hash;

            if count == longest_chain_len - 1 {
                break;
            }
            curr_block = self.chain.get(&parent_hash).unwrap().0.clone();
            count += 1;
        }
        list
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
    #[test]
    fn insert_fifty() {
        let mut blockchain = Blockchain::new();
        let mut count = 0;
        let mut curr_tip: H256;
        loop {
            if count == 50 {
                break;
            }
            curr_tip = blockchain.tip();
            let mut block = generate_random_block(&curr_tip);
            blockchain.insert(&block);
            count += 1;
        }
        println!("Chain: {:?}", blockchain.all_blocks_in_longest_chain());
        assert_eq!(blockchain.chain.len(), 51)
    }
    #[test]
    fn insert_fork() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);

        // fork by creating another block with the genesis hash as a parent
        let block_fork = generate_random_block(&genesis_hash);

        blockchain.insert(&block_fork);
        assert_eq!(blockchain.chain.get(&blockchain.tip()).unwrap().1, 1);
    }
    #[test]
    fn insert_long_fork() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        // fork by creating another block with the genesis hash as a parent
        let block_fork = generate_random_block(&genesis_hash);
        let second_block_fork: Block = generate_random_block(&block_fork.hash());
        blockchain.insert(&block_fork);
        assert_eq!(blockchain.tip(), block.hash());
        blockchain.insert(&second_block_fork);
        let vec = blockchain.all_blocks_in_longest_chain();
        // print entire hash
        let mut index = 0;
        println!("Longest Chain (ordered genesis to head):");
        loop {
            if (index == vec.len()) {
                break;
            }
            println!("{}", vec[index]);
            index += 1;
        }
        println!("Chain: {:?}", blockchain.all_blocks_in_longest_chain());
        assert_eq!(blockchain.tip(), second_block_fork.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
