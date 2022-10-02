use super::hash::{Hashable, H256};
use ring::digest;
#[derive(Debug, Clone, Default)]
pub struct Node<H256> {
    left_node: Option<Box<Node<H256>>>,
    right_node: Option<Box<Node<H256>>>,
    hash: H256,
}

/// A Merkle tree.
#[derive(Debug, Clone, Default)]
pub struct MerkleTree<T>
where
    T: Hashable,
{
    head: Node<T>,
    length: usize,
}

impl Node<H256> {
    pub fn new(
        left_node: Option<Box<Node<H256>>>,
        right_node: Option<Box<Node<H256>>>,
        hash: H256,
    ) -> Self {
        Node {
            left_node: left_node,
            right_node: right_node,
            hash: hash,
        }
    }
}
impl MerkleTree<H256> {
    pub fn new(data: &[H256]) -> Self {
        let mut count = 0;
        let mut nodes: Vec<Node<H256>> = vec![];
        let tree_length: usize;
        if data.len() == 0 {
            let zero_hash: H256 = [0; 32].into();
            let zero_node = Node {
                left_node: None,
                right_node: None,
                hash: zero_hash,
            };
            return MerkleTree {
                head: zero_node,
                length: 0,
            };
        }
        if data.len() % 2 != 0 {
            tree_length = data.len() + 1;
        } else {
            tree_length = data.len();
        }
        // create nodes of all data
        loop {
            if nodes.len() == data.len() {
                break;
            }
            nodes.push(Node::new(None, None, data[count].hash()));
            count += 1;
        }
        let mut index_current = 0;
        let mut index_next = 0;
        let mut len = nodes.len();
        loop {
            if len == 1 {
                break;
            }
            // clone the last node to make sure each level has an even number length
            if len % 2 != 0 {
                nodes.push(nodes[len - 1].clone());
                len += 1;
            }
            loop {
                if index_current + 1 >= len {
                    break;
                }
                nodes[index_next] = MerkleTree::hash_pair(
                    nodes[index_current].clone(),
                    nodes[index_current + 1].clone(),
                );
                index_current += 2;
                index_next += 1;
            }
            if len % 2 != 0 {
                len = len / 2 + 1;
            } else {
                len /= 2;
            }
            index_current = 0;
            index_next = 0;
        }
        MerkleTree {
            head: nodes[0].clone(),
            length: tree_length,
        }
    }
    fn hash_pair(left: Node<H256>, right: Node<H256>) -> Node<H256> {
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update(&left.hash.as_ref());
        ctx.update(&right.hash.as_ref());
        let new_hash: H256 = ctx.finish().into();
        Node {
            left_node: Option::Some(Box::new(left)),
            right_node: Option::Some(Box::new(right)),
            hash: new_hash,
        }
    }

    pub fn root(&self) -> H256 {
        self.head.hash
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut margin: usize = self.length / 2;
        let mut curr_node: Node<H256> = self.head.clone();
        let mut proof_of_data: Vec<H256> = vec![];
        loop {
            if margin == index {
                break;
            }
            if index >= margin {
                margin = margin + margin / 2;
                proof_of_data.push(curr_node.left_node.unwrap().hash);
                curr_node = *curr_node.right_node.unwrap();
            } else {
                margin = margin / 2;
                proof_of_data.push(curr_node.right_node.unwrap().hash);
                curr_node = *curr_node.left_node.unwrap();
            }
        }
        proof_of_data
    }
}
/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut new_hash: H256 = *datum;
    let mut level_size: usize;
    // in case we duplicated the last leaf to make a complete tree
    if leaf_size % 2 == 0 {
        level_size = leaf_size;
    } else {
        level_size = leaf_size + 1;
    }
    let mut level: usize = 0;
    loop {
        if level_size == 1 {
            break;
        }
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update(new_hash.as_ref());
        ctx.update(proof[(proof.len() - 1) - level].as_ref());
        new_hash = ctx.finish().into();
        level_size /= 2;
        level += 1;
    }
    new_hash == *root
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash::H256;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("375b0f2835fce8d166b048de307c1b726d60ca9abec3d5f7f0b47a4810ac5577")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(
            proof,
            vec![
                hex!("14b5bfc1bba8ef07311923e2ad5544d38ca752cd55fea4339531ed0f6ed434b6").into(),
                hex!("8c56ff4c190d4f6cd98b87661e77da02ce4c1436de294382278bfb915c30576c").into(),
                hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()
            ]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(
            &merkle_tree.root(),
            &input_data[0].hash(),
            &proof,
            0,
            input_data.len()
        ));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
