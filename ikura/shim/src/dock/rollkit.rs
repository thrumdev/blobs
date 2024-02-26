use std::{collections::HashMap, fmt};
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

use self::pbda::{
    da_service_server, Blob, CommitRequest, CommitResponse, GetIDsRequest, GetIDsResponse,
    GetProofsRequest, GetProofsResponse, GetRequest, GetResponse, MaxBlobSizeRequest,
    MaxBlobSizeResponse, SubmitRequest, SubmitResponse, ValidateRequest, ValidateResponse,
};

use crate::{ikura_rpc, key::Keypair};

pub mod pbda {
    tonic::include_proto!("da");
}

/// Configuration for the Rollkit dock.
pub struct Config {
    /// The RPC client handle to the ikura node.
    pub client: ikura_rpc::Client,

    /// The optional key used for signing when submitting blobs.
    pub submit_key: Option<Keypair>,

    /// The optional namespace to use, in case the namespace is not provided in the request.
    pub namespace: Option<ikura_nmt::Namespace>,

    /// The address to listen on.
    pub address: String,

    /// The port to listen on.
    pub port: u16,
}

/// Runs a gRPC Rollkit dock.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let Some(listen_on) = tokio::net::lookup_host((config.address.as_str(), config.port))
        .await?
        .next()
    else {
        anyhow::bail!(
            "failed to resolve address: {}:{}",
            config.address,
            config.port
        )
    };
    let dock = RollkitDock::new(config.client, config.submit_key, config.namespace);
    let service = da_service_server::DaServiceServer::new(dock);
    Server::builder()
        .add_service(service)
        .serve(listen_on)
        .await?;
    Ok(())
}

struct RollkitDock {
    client: ikura_rpc::Client,
    submit_key: Option<Keypair>,
    namespace: Option<ikura_nmt::Namespace>,
}

impl RollkitDock {
    fn new(
        client: ikura_rpc::Client,
        submit_key: Option<Keypair>,
        namespace: Option<ikura_nmt::Namespace>,
    ) -> Self {
        Self {
            client,
            submit_key,
            namespace,
        }
    }
}

#[tonic::async_trait]
impl da_service_server::DaService for RollkitDock {
    async fn max_blob_size(
        &self,
        request: Request<MaxBlobSizeRequest>,
    ) -> Result<Response<MaxBlobSizeResponse>, Status> {
        let MaxBlobSizeRequest {} = request.into_inner();
        const MAX_BLOB_SIZE: u64 = 100 * 1024;
        Ok(Response::new(MaxBlobSizeResponse {
            max_blob_size: MAX_BLOB_SIZE,
        }))
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let GetRequest { ids, namespace } = request.into_inner();
        // Deliberately ignore the namespace since blob ids uniquely identify the blobs.
        let _ = namespace;
        let mut cache = HashMap::new();
        let mut response = GetResponse { blobs: vec![] };
        for (index, id) in ids.into_iter().enumerate() {
            let blob_id = BlobId::try_from(id)
                .map_err(|_| Status::invalid_argument(format!("not a valid ID at {index}")))?;
            let block_number = blob_id.block_number;
            if !cache.contains_key(&block_number) {
                let block_hash = self.client.await_finalized_height(block_number).await;
                let block = self
                    .client
                    .await_block_at(Some(block_hash))
                    .await
                    .map_err(|_| {
                        Status::internal("failed to retrieve block number {block_number}")
                    })?;
                cache.insert(blob_id.block_number, block);
            }
            // unwrap: at this point we know the block is in the cache, because at this point
            // it must have been inserted into the cache or the block was already in the cache.
            let block = cache.get(&blob_id.block_number).unwrap();
            if let Some(needle) = block
                .blobs
                .iter()
                .find(|b| b.extrinsic_index == blob_id.extrinsic_index)
            {
                response.blobs.push(Blob {
                    value: needle.data.clone(),
                });
            } else {
                return Err(Status::not_found(format!("blob not found at {blob_id}")));
            }
        }
        Ok(Response::new(response))
    }

    // I know, the name is suboptimal, but it is what it is.
    async fn get_i_ds(
        &self,
        request: Request<GetIDsRequest>,
    ) -> Result<Response<GetIDsResponse>, Status> {
        let GetIDsRequest { namespace, height } = request.into_inner();
        let namespace = self.obtain_namespace(namespace)?;
        info!(
            "retrieving IDs from namespace '{}' at {}",
            &namespace, height
        );
        let block_hash = self.client.await_finalized_height(height).await;
        let Ok(block) = self.client.await_block_at(Some(block_hash)).await else {
            return Err(Status::internal("failed to retrieve block number {height}"));
        };

        // Collect all extrinsic indices for blobs in the given namespace.
        let mut ids = Vec::with_capacity(block.blobs.len());
        for blob in block.blobs {
            if blob.namespace == namespace {
                let blob_id = BlobId {
                    block_number: height,
                    extrinsic_index: blob.extrinsic_index,
                    data_hash: sha2_hash(&blob.data),
                };
                ids.push(blob_id.into());
            }
        }
        Ok(Response::new(GetIDsResponse { ids }))
    }

    async fn submit(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmitResponse>, Status> {
        let submit_key = self
            .submit_key
            .as_ref()
            .cloned()
            .ok_or_else(|| Status::failed_precondition("no key for signing blobs"))?;
        let SubmitRequest {
            namespace,
            blobs,
            gas_price: _,
        } = request.into_inner();
        let namespace = self.obtain_namespace(namespace)?;
        let mut response = SubmitResponse { ids: vec![] };
        let blob_n = blobs.len();
        for (i, blob) in blobs.into_iter().enumerate() {
            let data_hash = sha2_hash(&blob.value);
            info!(
                "submitting blob {i}/{blob_n} (0x{}) to namespace {}",
                hex::encode(&data_hash),
                namespace
            );
            let (block_hash, extrinsic_index) = self
                .client
                .submit_blob(blob.value, namespace, submit_key.clone())
                .await
                .map_err(|err| Status::internal(format!("failed to submit blob: {err}")))?;
            // TODO: getting the whole block is a bit inefficient, consider optimizing.
            let block_number = match self
                .client
                .await_block_at(Some(block_hash))
                .await
                .map(|block| block.number)
            {
                Ok(block_number) => block_number,
                Err(err) => {
                    return Err(Status::internal(format!(
                        "failed to obtain block number for 0x{}: {:?}",
                        hex::encode(&block_hash),
                        err,
                    )));
                }
            };
            let blob_id = BlobId {
                block_number,
                extrinsic_index,
                data_hash,
            };
            info!("blob landed: {blob_id}");
            response.ids.push(blob_id.into());
        }
        Ok(Response::new(response))
    }

    async fn get_proofs(
        &self,
        request: Request<GetProofsRequest>,
    ) -> Result<Response<GetProofsResponse>, Status> {
        // TODO: implement
        // https://github.com/thrumdev/blobs/issues/257
        let GetProofsRequest { ids, .. } = request.into_inner();
        let response = GetProofsResponse {
            proofs: ids
                .into_iter()
                .map(|_| pbda::Proof { value: vec![] })
                .collect(),
        };
        Ok(Response::new(response))
    }

    async fn validate(
        &self,
        request: Request<ValidateRequest>,
    ) -> Result<Response<ValidateResponse>, Status> {
        // TODO: implement
        // https://github.com/thrumdev/blobs/issues/257
        let ValidateRequest { ids, .. } = request.into_inner();
        let response = ValidateResponse {
            results: ids.into_iter().map(|_| true).collect(),
        };
        Ok(Response::new(response))
    }

    async fn commit(
        &self,
        request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        // TODO: implement
        // https://github.com/thrumdev/blobs/issues/257
        let CommitRequest { blobs, .. } = request.into_inner();
        let response = CommitResponse {
            commitments: blobs
                .into_iter()
                .map(|_| pbda::Commitment { value: vec![] })
                .collect(),
        };
        Ok(Response::new(response))
    }
}

impl RollkitDock {
    /// Returns the namespace to be used, either from the request or from the configuration.
    ///
    /// If the namespace is not provided in the request, it will use the namespace from the
    /// configuration.
    fn obtain_namespace(
        &self,
        supplied_ns: Option<pbda::Namespace>,
    ) -> Result<ikura_nmt::Namespace, Status> {
        Ok(match supplied_ns {
            Some(pbda::Namespace {
                value: raw_namespace_bytes,
            }) => {
                let raw_namespace_bytes = raw_namespace_bytes.as_slice();
                if raw_namespace_bytes.len() != 16 {
                    return Err(Status::invalid_argument("namespace must be 16 bytes long"));
                }
                let mut namespace = [0u8; 16];
                namespace.copy_from_slice(raw_namespace_bytes);
                ikura_nmt::Namespace::from_raw_bytes(namespace)
            }
            None => {
                if let Some(namespace) = &self.namespace {
                    namespace.clone()
                } else {
                    return Err(Status::invalid_argument("namespace must be provided"));
                }
            }
        })
    }
}

fn sha2_hash(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(data).into()
}

struct BlobId {
    /// The block number at which the blob in question has been landed.
    block_number: u64,
    /// The index of extrinsic in the block, specified by the block_number.
    ///
    /// The extrinsic should be of `submit_blob` call.
    extrinsic_index: u32,
    /// The sha256 hash of the blob's contents.
    data_hash: [u8; 32],
}

impl BlobId {
    const BINARY_SZ: usize = 44;
}

#[derive(Debug)]
pub enum TryFromRawIdError {
    InvalidLength,
}

impl TryFrom<pbda::Id> for BlobId {
    type Error = TryFromRawIdError;
    fn try_from(id: pbda::Id) -> Result<Self, TryFromRawIdError> {
        let buf = id.value;
        let sz = buf.len();
        if sz != Self::BINARY_SZ {
            return Err(TryFromRawIdError::InvalidLength);
        }
        // unwrap: the buffer is guaranteed to be of the correct size.
        let block_number: [u8; 8] = buf[0..8].try_into().unwrap();
        let extrinsic_index: [u8; 4] = buf[8..12].try_into().unwrap();
        let data_hash: [u8; 32] = buf[12..].try_into().unwrap();

        let block_number = u64::from_be_bytes(block_number);
        let extrinsic_index = u32::from_be_bytes(extrinsic_index);

        Ok(Self {
            block_number,
            extrinsic_index,
            data_hash,
        })
    }
}

impl From<BlobId> for pbda::Id {
    fn from(blob_id: BlobId) -> Self {
        // Serializes the block number and extrinsic index into a buffer.
        let mut buf = Vec::with_capacity(44);
        buf.extend_from_slice(&blob_id.block_number.to_be_bytes());
        buf.extend_from_slice(&blob_id.extrinsic_index.to_be_bytes());
        buf.extend_from_slice(&blob_id.data_hash);
        pbda::Id { value: buf }
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            self.block_number,
            self.extrinsic_index,
            hex::encode(&self.data_hash)
        )
    }
}
