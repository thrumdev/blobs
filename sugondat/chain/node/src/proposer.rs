//! Wrapper around a proposer which only authors blocks when certain conditions are met.
//!
//! These conditions are:
//! 1. There is at least one transaction ready to post. This is used to determine that there were
//!    non-inherent extrinsics and avoid authoring empty blocks.
//! 2. There is an incoming downward message from the relay chain.
//! 3. There is a go-ahead signal for a parachain code upgrade.
//! 4. The block is the first block of the parachain. Useful for testing.
//! 5. The chain is not producing blocks for more than the maximum allowed number of skipped blocks
//!
//! If any of these conditions are met, then the block is authored.

use anyhow::anyhow;
use parity_scale_codec::Decode;

use sc_client_api::backend::StorageProvider;
use sc_transaction_pool_api::TransactionPool;
use sp_api::StorageProof;
use sp_consensus::Proposal;
use sp_inherents::InherentData;
use sp_runtime::generic::Digest;

use cumulus_client_consensus_proposer::{Error as ProposerError, ProposerInterface};
use cumulus_pallet_parachain_system::relay_state_snapshot::RelayChainStateProof;
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::ParachainInherentData;

use sugondat_primitives::{
    opaque::{Block, Header},
    MAX_SKIPPED_BLOCKS,
};

use std::sync::Arc;
use std::time::Duration;

use crate::service::ParachainClient;

/// Proposes blocks, but only under certain conditions. See module docs.
pub struct BlockLimitingProposer<P, C> {
    inner: P,
    client: Arc<C>,
    para_id: ParaId,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
}

impl<P, C> BlockLimitingProposer<P, C> {
    /// Create a new block-limiting proposer.
    pub fn new(
        inner: P,
        client: Arc<C>,
        para_id: ParaId,
        transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
    ) -> Self {
        BlockLimitingProposer {
            inner,
            client,
            para_id,
            transaction_pool,
        }
    }
}

#[async_trait::async_trait]
impl<P, C> ProposerInterface<Block> for BlockLimitingProposer<P, C>
where
    P: ProposerInterface<Block> + Send,
    C: StorageProvider<Block, sc_client_db::Backend<Block>> + Send + Sync,
{
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
        let first_block = {
            // allow creating the first block without the above conditions. This is useful for
            // testing for detection of healthiness.
            parent_header.number == 0
        };
        let exceeded_max_skipped_blocks = 'max_skipped: {
            let maybe_last_relay_block_number = self.client.storage(
                parent_header.parent_hash,
                &sp_storage::StorageKey(sugondat_primitives::last_relay_block_number_key()),
            );

            // If the state of the previous block or the last relay block number
            // is not available, to be sure of not exceeding the max amount of
            // skippable blocks, the block will be produced.
            let last_relay_block_number = match maybe_last_relay_block_number {
                Ok(Some(raw_data)) => match Decode::decode(&mut &raw_data.0[..]) {
                    Ok(last_relay_block_number) => last_relay_block_number,
                    Err(_) => break 'max_skipped true,
                },
                _ => break 'max_skipped true,
            };
            let relay_block_number = paras_inherent_data.validation_data.relay_parent_number;

            let relay_parent_distance = relay_block_number.saturating_sub(last_relay_block_number);
            let n_skipped_blocks = relay_parent_distance.saturating_sub(2) / 2;
            n_skipped_blocks >= MAX_SKIPPED_BLOCKS
        };

        if has_downward_message
            || has_go_ahead
            || has_transactions
            || first_block
            || exceeded_max_skipped_blocks
        {
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
