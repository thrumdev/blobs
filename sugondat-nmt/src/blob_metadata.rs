use crate::leaf::NmtLeaf;
use crate::ns::Namespace;

#[derive(Debug, Clone)]
pub struct BlobMetadata {
    pub namespace: Namespace,
    pub leaf: NmtLeaf,
}
