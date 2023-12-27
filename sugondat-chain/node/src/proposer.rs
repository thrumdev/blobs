//! Wrapper around a proposer which only authors blocks when certain conditions are met.
//!
//! These conditions are:
//! 1. There is at least one transaction ready to post. This is used to determine that there were
//!    non-inherent extrinsics and avoid authoring empty blocks.
//! 2. There is an incoming downward message from the relay chain.
//! 3. There is a go-ahead signal for a parachain code upgrade.
//!
//! If any of these conditions are met, then the block is authored.

use anyhow::anyhow;

use sc_transaction_pool_api::TransactionPool;
use sp_api::StorageProof;
use sp_consensus::Proposal;
use sp_inherents::InherentData;
use sp_runtime::generic::Digest;

use cumulus_client_consensus_proposer::{Error as ProposerError, ProposerInterface};
use cumulus_pallet_parachain_system::relay_state_snapshot::RelayChainStateProof;
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::ParachainInherentData;

use sugondat_primitives::opaque::{Block, Header};

use std::sync::Arc;
use std::time::Duration;

use crate::service::ParachainClient;

/// Proposes blocks, but only under certain conditions. See module docs.
pub struct BlockLimitingProposer<P> {
    inner: P,
    para_id: ParaId,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
}

#[async_trait::async_trait]
impl<P: ProposerInterface<Block> + Send> ProposerInterface<Block> for BlockLimitingProposer<P> {
    async fn propose(
        &mut self,
        parent_header: &Header,
        paras_inherent_data: &ParachainInherentData,
        other_inherent_data: InherentData,
        inherent_digests: Digest,
        max_duration: Duration,
        block_size_limit: Option<usize>,
    ) -> Result<Proposal<Block, StorageProof>, ProposerError> {
        let has_downward_message = !paras_inherent_data.downward_messages.is_empty();
        let has_transactions = self.transaction_pool.status().ready > 0;
        let has_go_ahead = {
            let maybe_go_ahead = RelayChainStateProof::new(
                self.para_id,
                paras_inherent_data
                    .validation_data
                    .relay_parent_storage_root,
                paras_inherent_data.relay_chain_state.clone(),
            )
            .and_then(|p| p.read_upgrade_go_ahead_signal());

            // when we encounter errors reading the go ahead signal,
            // we pessimistically opt to create a block as such errors indicate
            // changes in the relay chain protocol and would likely persist
            // until something is fixed here.
            match maybe_go_ahead {
                Err(_) => true,
                Ok(Some(_)) => true,
                Ok(None) => false,
            }
        };

        if has_downward_message || has_go_ahead || has_transactions {
            self.inner
                .propose(
                    parent_header,
                    paras_inherent_data,
                    other_inherent_data,
                    inherent_digests,
                    max_duration,
                    block_size_limit,
                )
                .await
        } else {
            Err(ProposerError::proposing(anyhow!(
                "no need to create a block"
            )))
        }
    }
}
