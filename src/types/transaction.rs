use super::address::Address;
use super::hash::{Hashable, H256};
use crate::blockchain::State;
use std::convert::TryInto;
use rand::{thread_rng, Rng};
use super::key_pair;
use ring::signature::{
    Ed25519KeyPair, EdDSAParameters, KeyPair, Signature, UnparsedPublicKey, VerificationAlgorithm,
    ED25519,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub sender: Address,
    pub nonce : usize,
    pub receiver: Address,
    pub value: usize,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub t: Transaction,
    pub sig: Vec<u8>,
    pub pub_key: Vec<u8>,
}
impl SignedTransaction{
    pub fn verify(&self, curr_state: &State) -> bool{
        let peer_public_key = UnparsedPublicKey::new(&ED25519, self.pub_key.clone());
        let hash : [u8; 32] = self.t.hash().into();
        if !peer_public_key.verify(&hash, &self.sig).is_ok(){
            return false
        }
        let sender = self.t.sender;
        let receiver = self.t.receiver;
        let accounts = curr_state.get_accounts();
        // check if the blockchain contains these two accounts
        // if the sender account isn't in the chain, then the transaction(and therefore block) is invalid
        if !accounts.contains_key(&sender) {
            return false
        }

        let account_nonce = self.t.nonce;
        let value = self.t.value;
        let sig = self.sig.clone();
        let key = self.pub_key.clone();
        let (sender_nonce, sender_bal) = accounts.get(&sender).unwrap();
        // check sender has enough funds and account_nonce has only been incremented once
        if sender_bal > &value && (sender_nonce + 1) == account_nonce {
            // println!("Sender Balance: {:?}", sender_bal);
            // println!("Value of Transaction: {:?}", value);
            return true;
        }
        false
    }
}
impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let bytes: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}
impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let bytes: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}
/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    // first generate a key pair(private key, public key)
    let hash : [u8; 32] = t.hash().into();
    key.sign(&hash)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8], state: State) -> bool {
    let peer_public_key = UnparsedPublicKey::new(&ED25519, public_key);
    let hash : [u8; 32] = t.hash().into();
    peer_public_key.verify(&hash, signature).is_ok()
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    let mut count = 0;
    let mut addr_arr_sender: [u8; 20] = [0; 20];
    let mut addr_arr_receiver: [u8; 20] = [0; 20];

    let mut rng = thread_rng();

    loop {
        addr_arr_sender[count] = rng.gen();
        addr_arr_receiver[count] = rng.gen();

        if (count >= 19) {
            break;
        }
        count += 1;
    }
    let addr_sender = Address::new(addr_arr_sender);
    let addr_receiver = Address::new(addr_arr_receiver);
    let val: u8 = rng.gen();
    Transaction {
        sender: addr_sender,
        receiver: addr_receiver,
        value: val,
    }
}
pub fn generate_signed_transaction(key_pair_sender: &Ed25519KeyPair, receiver_addr: &Address, nonce: &usize, bal: &usize) -> SignedTransaction {
    let mut rng = thread_rng();
    let pub_key_sender = key_pair_sender.public_key();

    let addr_sender = Address::from_public_key_bytes(pub_key_sender.as_ref());

    let val: usize = rng.gen_range(1..=100);

    let t = Transaction {
        sender: addr_sender,
        receiver: *receiver_addr,
        nonce: nonce + 1,
        value: val
    };
    let sig = sign(&t, &key_pair_sender).as_ref().to_vec();
    SignedTransaction{
        t: t, 
        sig: sig, 
        pub_key: pub_key_sender.as_ref().to_vec()
    }
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
