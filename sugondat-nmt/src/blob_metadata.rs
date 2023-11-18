use crate::{leaf::NmtLeaf, ns::Namespace};

#[derive(Debug, Clone)]
pub struct BlobMetadata {
    pub namespace: Namespace,
    pub leaf: NmtLeaf,
}
