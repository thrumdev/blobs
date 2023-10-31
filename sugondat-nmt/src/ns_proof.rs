use crate::ns::Namespace;
use crate::{NmtLeaf, TreeRoot, NS_ID_SIZE};
use alloc::vec::Vec;

#[derive(Debug)]
pub enum VerifyErr {
    BlobCountMismatch,
    MalformedLeaf(usize),
    BlobHashMismatch(usize),
    VerifyProof,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NamespaceProof {
    pub(crate) leaves: Vec<Vec<u8>>,
    pub(crate) proof: nmt_rs::NamespaceProof<nmt_rs::NamespacedSha2Hasher<NS_ID_SIZE>, NS_ID_SIZE>,
}

impl NamespaceProof {
    pub fn verify(
        self,
        blob_hashes: &[[u8; 32]],
        root: TreeRoot,
        namespace: Namespace,
    ) -> Result<(), VerifyErr> {
        if blob_hashes.len() != self.leaves.len() {
            return Err(VerifyErr::BlobCountMismatch);
        }
        let root = nmt_rs::NamespacedHash::<NS_ID_SIZE>::new(
            root.min_ns.nmt_namespace_id(),
            root.max_ns.nmt_namespace_id(),
            root.root,
        );
        self.proof
            .verify_complete_namespace(&root, &self.leaves, namespace.nmt_namespace_id())
            .map_err(|_| VerifyErr::VerifyProof)?;
        for (i, leaf) in self.leaves.iter().enumerate() {
            if leaf.len() != 68 {
                return Err(VerifyErr::MalformedLeaf(i));
            }
            let leaf = NmtLeaf::from_raw_bytes(leaf.as_slice().try_into().unwrap());
            if leaf.blob_hash != blob_hashes[i] {
                return Err(VerifyErr::BlobHashMismatch(i));
            }
        }
        Ok(())
    }
}
