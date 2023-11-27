use crate::{leaf::NmtLeaf, ns::Namespace, ns_proof::NamespaceProof, root::TreeRoot, NS_ID_SIZE};

use nmt_rs::{simple_merkle::db::MemDb, NamespaceMerkleTree, NamespacedHash, NamespacedSha2Hasher};

#[derive(Debug)]
pub enum PushLeafErr {
    /// The namespace is not in ascending order.
    AscendingOrder,
}

pub struct TreeBuilder {
    tree: NamespaceMerkleTree<
        MemDb<NamespacedHash<NS_ID_SIZE>>,
        NamespacedSha2Hasher<NS_ID_SIZE>,
        NS_ID_SIZE,
    >,
    last_namespace: Namespace,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            tree: NamespaceMerkleTree::new(),
            last_namespace: Namespace::from_u32_be(0),
        }
    }

    pub fn push_leaf(
        &mut self,
        namespace: Namespace,
        nmt_leaf: NmtLeaf,
    ) -> Result<(), PushLeafErr> {
        if namespace < self.last_namespace {
            return Err(PushLeafErr::AscendingOrder);
        }
        self.last_namespace = namespace;
        let leaf = nmt_leaf.to_raw_bytes();
        self.tree
            .push_leaf(&leaf, namespace.nmt_namespace_id())
            .expect("the error is manually checked above");

        Ok(())
    }

    pub fn root(&mut self) -> TreeRoot {
        let root = self.tree.root();
        let min_ns = Namespace::with_nmt_namespace_id(root.min_namespace());
        let max_ns = Namespace::with_nmt_namespace_id(root.max_namespace());
        TreeRoot {
            root: root.hash(),
            min_ns,
            max_ns,
        }
    }

    pub fn proof(&mut self, namespace: Namespace) -> NamespaceProof {
        let (leaves, proof) = self
            .tree
            .get_namespace_with_proof(namespace.nmt_namespace_id());
        NamespaceProof { leaves, proof }
    }
}
