use super::address::Address;
use super::transaction::{SignedTransaction, Transaction};
use crate::types::hash::{Hashable, H256};
use bincode;
use rand::{thread_rng, Rng};
use ring::digest;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    header: Header,
    content: Content,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    parent: H256,
    nonce: u32,
    difficulty: H256,
    timestamp: SystemTime,
    merkle_root: H256,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Content {
    content: Vec<SignedTransaction>,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}
impl Hashable for Header {
    fn hash(&self) -> H256 {
        // do i need to hash the individual parts before adding them to context?
        // let nonce_bytes: Vec<u8> = bincode::serialize(&self.nonce).unwrap();
        let header_serialized: Vec<u8> = bincode::serialize(&self).unwrap();
        let new_hash: H256 = ring::digest::digest(&ring::digest::SHA256, &header_serialized).into();
        new_hash
    }
}
impl Block {
    pub fn new(
        parent: H256,
        nonce: u32,
        timestamp: SystemTime,
        difficulty: H256,
        merkle_root: H256,
        content: Vec<SignedTransaction>,
    ) -> Self {
        let header = Header {
            parent: parent,
            nonce: nonce,
            timestamp: timestamp,
            difficulty: difficulty,
            merkle_root: merkle_root,
        };
        let content = Content { content };
        Self { header, content }
    }
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    let mut rng = thread_rng();

    // random nonce
    let nonce: u32 = rng.gen();
    // empty merkle root -
    let mut empty_root: H256 = [0; 32].into();
    empty_root = empty_root.hash();
    // arbitrary difficulty
    let difficulty: H256 = [5; 32].into();
    // current timestamp
    let timestamp = SystemTime::now();

    let header = Header {
        parent: *parent,
        nonce: nonce,
        timestamp: timestamp,
        difficulty: difficulty,
        merkle_root: empty_root,
    };
    let content = Content { content: vec![] };

    Block {
        header: header,
        content: content,
    }
}
