use super::address::Address;
use super::hash::{Hashable, H256};
use rand::{thread_rng, Rng};
use super::key_pair;
use ring::signature::{
    Ed25519KeyPair, EdDSAParameters, KeyPair, Signature, UnparsedPublicKey, VerificationAlgorithm,
    ED25519,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    sender: Address,
    receiver: Address,
    value: u8,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    t: Transaction,
    sig: Vec<u8>,
    pub_key: Vec<u8>,
}
impl SignedTransaction{
    pub fn verify(&self) -> bool{
        let peer_public_key = UnparsedPublicKey::new(&ED25519, self.pub_key.clone());
        peer_public_key.verify(&[self.t.value], &self.sig).is_ok()
    }
}
impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let bytes: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}
/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    // first generate a key pair(private key, public key)
    key.sign(&[t.value])
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let peer_public_key = UnparsedPublicKey::new(&ED25519, public_key);

    peer_public_key.verify(&[t.value], signature).is_ok()
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
pub fn generate_signed_transaction() -> SignedTransaction{
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
    let t = Transaction {
        sender: addr_sender,
        receiver: addr_receiver,
        value: val,
    };
    let key = key_pair::random();
    let sig = sign(&t, &key).as_ref().to_vec();
    let pub_key : Vec<u8> = key.public_key().as_ref().to_vec();
    SignedTransaction{t, sig, pub_key}
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
