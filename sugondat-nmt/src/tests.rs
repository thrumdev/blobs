use crate::{tree_from_blobs, BlobMetadata, Namespace, NmtLeaf, TreeBuilder};
use alloc::vec::Vec;

struct MockBuilder {
    blobs: Vec<BlobMetadata>,
}

impl MockBuilder {
    fn new() -> Self {
        Self { blobs: Vec::new() }
    }

    fn push_blob(&mut self, who: [u8; 32], namespace: Namespace, blob_hash: [u8; 32]) {
        let index = self.blobs.len();
        self.blobs.push(BlobMetadata {
            namespace,
            leaf: NmtLeaf {
                extrinsic_index: index as u32,
                who,
                blob_hash,
            },
        });
    }

    fn tree(&self) -> TreeBuilder {
        tree_from_blobs(self.blobs.clone())
    }
}

#[test]
fn two_same_blobs() {
    let mut b = MockBuilder::new();
    b.push_blob([1u8; 32], Namespace::from_u32_be(1), [2u8; 32]);
    b.push_blob([1u8; 32], Namespace::from_u32_be(1), [2u8; 32]);
    let mut tree = b.tree();
    let proof = tree.proof(Namespace::from_u32_be(1));
    assert!(proof
        .verify(
            &[[2u8; 32], [2u8; 32]],
            tree.root(),
            Namespace::from_u32_be(1)
        )
        .is_ok());
}

#[test]
fn empty() {
    let mut tree = MockBuilder::new().tree();
    let proof = tree.proof(Namespace::from_u32_be(1));
    assert!(proof
        .verify(&[], tree.root(), Namespace::from_u32_be(1))
        .is_ok());
}

#[test]
fn empty_absent_namespace_id() {
    let mut tree = MockBuilder::new().tree();
    let proof = tree.proof(Namespace::from_u32_be(1));
    assert!(proof
        .verify(&[], tree.root(), Namespace::from_u32_be(2))
        .is_ok());
}

#[test]
fn proof_absent_namespace_id() {
    let mut b = MockBuilder::new();
    b.push_blob([1u8; 32], Namespace::from_u32_be(1), [2u8; 32]);
    let mut tree = b.tree();
    let proof = tree.proof(Namespace::from_u32_be(2));
    assert!(proof
        .verify(&[], tree.root(), Namespace::from_u32_be(2))
        .is_ok());
}

#[test]
fn wrong_namespace_id() {
    let mut b = MockBuilder::new();
    b.push_blob([1u8; 32], Namespace::from_u32_be(1), [2u8; 32]);
    let mut tree = b.tree();
    let proof = tree.proof(Namespace::from_u32_be(2));
    assert!(proof
        .verify(&[], tree.root(), Namespace::from_u32_be(1))
        .is_err());
}
