use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    chain: HashMap<H256, Block>,
    head: Block,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut rng = thread_rng();
        // random nonce(doesn't have to solve the puzzle for the genesis according to Office Hours)
        let nonce: u32 = rng.gen();

        // random parent(okay according to Office Hours)
        let mut parent: H256 = [0; 32].into();
        // empty root as well, genesis stores no data right now
        let mut merkle_root: H256 = [0; 32].into();
        merkle_root = merkle_root.hash();
        // arbitrary difficulty
        let difficulty: H256 = [1; 32].into();
        // current timestamp
        let timestamp = SystemTime::now();
        let height = 0;
        let mut chain: HashMap<H256, Block> = HashMap::new();
        let genesis = Block::new(
            parent,
            nonce,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            difficulty,
            merkle_root,
            vec![],
            height,
        );
        chain.insert(genesis.hash(), genesis.clone());
        // OFFICE HOURS: SHOULD I CLONE HERE?
        Self {
            chain: chain,
            head: genesis.clone(),
        }
    }
    pub fn get_block(&self, block_hash: &H256) -> Block {
        let block: Block = self.chain.get(block_hash).unwrap().clone();
        block
    }
    pub fn get_height(&self, block: &Block) -> usize {
        // let genesis_hash: H256 = [0; 32].into();
        // let mut curr_block = block.clone();
        // let mut height = 0;
        // loop {
        //     if curr_block.get_parent() == genesis_hash {
        //         break;
        //     }
        //     height += 1;
        //     curr_block = self.get_block(&curr_block.get_parent());
        // }
        block.get_height()
    }
    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        // let tip_height: usize = self.get_height(&self.head);
        let tip_height: usize = self.get_tip_height();
        let block_height: usize = block.get_height();
        self.chain.insert(block.hash(), block.clone());
        // rule = only make the fork the new longest chain if the fork tip is strictly longer than the current tip
        if self.tip() == block.get_parent() || block_height > tip_height {
            self.head = block.clone();
        }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.head.hash()
    }
    pub fn get_tip_height(&self) -> usize {
        self.head.get_height()
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
            curr_block = self.chain.get(&parent_hash).unwrap().clone();
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
        let mut block = generate_random_block(&genesis_hash);
        block.set_height(blockchain.get_block(&block.get_parent()).get_height() + 1);
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
            block.set_height(blockchain.get_block(&block.get_parent()).get_height() + 1);

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
        let mut block = generate_random_block(&genesis_hash);
        block.set_height(blockchain.get_block(&block.get_parent()).get_height() + 1);

        blockchain.insert(&block);
        // fork by creating another block with the genesis hash as a parent
        let mut block_fork = generate_random_block(&genesis_hash);
        block_fork.set_height(blockchain.get_block(&block.get_parent()).get_height() + 1);

        blockchain.insert(&block_fork);
        assert_eq!(blockchain.chain.len(), 2);
    }
    #[test]
    fn insert_long_fork() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let mut block = generate_random_block(&genesis_hash);
        block.set_height(blockchain.get_block(&block.get_parent()).get_height() + 1);
        blockchain.insert(&block);
        // fork by creating another block with the genesis hash as a parent
        let mut block_fork = generate_random_block(&genesis_hash);
        block_fork.set_height(blockchain.get_block(&block_fork.get_parent()).get_height() + 1);
        let mut second_block_fork: Block = generate_random_block(&block_fork.hash());
        second_block_fork.set_height(second_block_fork.get_height() + 1);
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
