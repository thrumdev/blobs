//! This crate provides utilities for working with the namespaced merkle tree (NMT) used in Ikura.

#![no_std]

extern crate alloc;

pub const NS_ID_SIZE: usize = 16;

mod blob_metadata;
mod leaf;
mod ns;
mod ns_proof;
mod root;
mod tree;

#[cfg(test)]
mod tests;

pub use blob_metadata::BlobMetadata;
pub use leaf::NmtLeaf;
pub use ns::Namespace;
pub use ns_proof::NamespaceProof;
pub use root::TreeRoot;
pub use tree::{PushLeafErr, TreeBuilder};

use alloc::vec::Vec;

/// Creates a namespaced merkle tree from the list of blob metadata.
pub fn tree_from_blobs(mut blob_metadata: Vec<BlobMetadata>) -> TreeBuilder {
    blob_metadata.sort_by_key(|blob| blob.namespace);

    let mut tree = TreeBuilder::new();
    for blob in blob_metadata {
        match tree.push_leaf(blob.namespace, blob.leaf) {
            Ok(()) => (),
            Err(PushLeafErr::AscendingOrder) => {
                panic!("sorted by namespace, so this should not happen")
            }
        }
    }
    tree
}
