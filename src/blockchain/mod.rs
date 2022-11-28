use crate::types::block::{Block, Content, Header};
use crate::types::hash::{Hashable, H256};
use crate::types::transaction::SignedTransaction;
use crate::types::address::Address;
use crate::types::key_pair;
use ring::signature::{KeyPair, Ed25519KeyPair};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    chain: HashMap<H256, (Block, usize)>,
    head: Block,
    pub block_state: HashMap<H256, State>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub accounts: HashMap<Address, (usize, usize)>
}
impl State {
    pub fn new() -> Self {
        let accounts : HashMap<Address, (usize,usize)> = HashMap::new();
        Self{accounts}
    }
    pub fn add_account(&mut self, account_addr: Address, bal:usize) {
        self.accounts.insert(account_addr.clone(), (0, bal));
    }
    pub fn get_accounts(&self) -> HashMap<Address, (usize, usize)>{
        self.accounts.clone()
    }
    pub fn contains(&self, address: &Address) -> bool{
        self.accounts.contains_key(address)
    }

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
        let difficulty: H256 = [1; 32].into();
        println!("difficulty: {:?}", difficulty);
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
        let seed_1 : [u8; 32] = [0; 32];
        let seed_2 : [u8;32] = [1; 32];
        let seed_3 : [u8;32] = [2; 32];

        let key_pair_1 = key_pair::from_seed(seed_1);
        let key_pair_2 = key_pair::from_seed(seed_2);
        let key_pair_3 = key_pair::from_seed(seed_3);
        // ico
        let mut state = State::new();
        let pub_key_1 = key_pair_1.public_key().as_ref();
        let account_addr_1 = Address::from_public_key_bytes(pub_key_1);
        state.add_account(account_addr_1, 10000);
        let pub_key_2 = key_pair_2.public_key().as_ref();
        let account_addr_2 = Address::from_public_key_bytes(pub_key_2);
        state.add_account(account_addr_2, 0);
        let pub_key_3 = key_pair_3.public_key().as_ref();
        let account_addr_3 = Address::from_public_key_bytes(pub_key_3);
        state.add_account(account_addr_3, 0);         
        
        println!("Accounts: {:?}", state.get_accounts());
        let mut block_state: HashMap<H256, State> = HashMap::new();
        block_state.insert(genesis.hash(), state);
        chain.insert(genesis.hash(), (genesis.clone(), height));
        Self {
            chain: chain,
            head: genesis.clone(),
            block_state: block_state.clone()
        }
    }
    pub fn update_state(&mut self, block: &Block) -> bool {
        let mut prev_state : State = self.block_state.get(&block.get_parent()).unwrap().clone();
        let mut valid_block = true;
        for transaction in block.get_content(){
            let sender = transaction.t.sender;
            let receiver = transaction.t.receiver;
            // add receivers not in the state yet that are being sent coins
            if !prev_state.contains(&receiver){
                prev_state.add_account(receiver, 0);
            }
            let accounts = prev_state.get_accounts();
            let (sender_nonce, sender_bal) = accounts.get(&sender).unwrap();
            let (receiver_nonce, receiver_bal) = accounts.get(&receiver).unwrap();
            let value = transaction.t.value;
            if sender_bal < &value{
                valid_block = false;
                break;
            }
            prev_state.accounts.insert(sender, (*sender_nonce + 1, sender_bal - value));
            prev_state.accounts.insert(receiver, (*receiver_nonce, receiver_bal + value));
        }
            // add block, state to the block_state
        if valid_block {
            self.block_state.insert(block.hash(), prev_state.clone());
        }
        valid_block
    }
    pub fn contains(&self, key: &H256) -> bool{
        self.chain.contains_key(key)
    }
    pub fn get_state(&mut self, block: &Block) -> State{
        self.block_state.get(&block.hash()).unwrap().clone()
    }
    pub fn get_tip_state(&mut self) -> State{
        self.get_state(&self.head())
    }
    pub fn verify_block(&mut self, block: &Block) -> bool{
        // if a block makes it here, it's parent is known to be in the chain
        let parent = self.get_block(&block.get_parent()).unwrap();
        if !(block.get_difficulty() == parent.get_difficulty()) {
            return false;
        }
        let mut curr_state = self.get_tip_state();
        if block.get_content().is_empty() {
            return false;
        }
        for transaction in block.get_content(){
            // if the receiver account isn't in the chain, add a receiver account
            let receiver = transaction.t.receiver;
            if !curr_state.contains(&receiver){
                curr_state.add_account(receiver, 0);
            }
            if !transaction.verify(&curr_state){
                return false;
            }
        }
        return true;
    }
    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let tip_height: usize = self.get_tip_height();
        let parent_height: usize = self.chain.get(&block.get_parent()).unwrap().1;
        let block_height: usize = parent_height + 1;
        self.chain.insert(block.hash(), (block.clone(), block_height));
        // rule = only make the fork the new longest chain if the fork tip is strictly longer than the current tip
        if self.tip() == block.get_parent() || block_height > tip_height {
            self.head = block.clone();
        }
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
        let mut count = 0;
        let longest_chain_len: usize = self.get_tip_height() + 1;
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
    pub fn all_transactions_in_longest_chain(&self) -> Vec<Vec<SignedTransaction>> {
        let mut curr_block: Block = self.head.clone();
        let mut parent_hash: H256;
        let mut count = 0;
        let longest_chain_len: usize = self.get_tip_height() + 1;
        let empty_vec : Vec<SignedTransaction> = vec![];
        let mut list: Vec<Vec<SignedTransaction>> = vec![empty_vec.clone(); longest_chain_len];
        
        { loop {
            if count == longest_chain_len - 1 {
                break;
            }
            parent_hash = curr_block.get_parent();
            list[longest_chain_len - 1 - count] = curr_block.get_content();
            curr_block = self.chain.get(&parent_hash).unwrap().0.clone();
            count += 1;
        } }
        list
    }
    pub fn state_at_block(&self, block_num: usize) -> Vec<(Address, usize, usize)> {
        let longest_chain_len: usize = self.get_tip_height() + 1;
        let mut len = longest_chain_len.clone();
        let mut curr_block = self.head();
        let mut parent_hash: H256;

        { loop {
            if len == block_num {
                break;
            }
            parent_hash = curr_block.get_parent();
            curr_block = self.chain.get(&parent_hash).unwrap().0.clone();
            len -= 1;
        } }
        let state : &State = &self.block_state.get(&curr_block.hash()).unwrap().clone();
        let mut state_vec : Vec<(Address, usize, usize)> = vec![];
        for (account, value) in &state.accounts{
            state_vec.push((account.clone(), value.0.clone(), value.1.clone()));
        }
        state_vec
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
        loop {
            if (index == vec.len()) {
                break;
            }
            index += 1;
        }
        assert_eq!(blockchain.tip(), second_block_fork.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
