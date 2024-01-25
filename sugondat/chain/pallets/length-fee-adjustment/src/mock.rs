use crate as pallet_sugondat_length_fee_adjustment;
use frame_support::{
    parameter_types,
    traits::ConstU64,
    weights::{Weight, WeightToFee as WeightToFeeT},
};
use pallet_transaction_payment::Multiplier;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    FixedPointNumber, SaturatedConversion,
};

use pallet_sugondat_length_fee_adjustment::LastRelayBlockNumberProvider;
use polkadot_primitives::v6::BlockNumber as RelayChainBlockNumber;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
        LengthFeeAdjustment: pallet_sugondat_length_fee_adjustment::{Pallet, Storage},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type MaxHolds = ();
}

parameter_types! {
    pub TransactionByteFee: Balance = 333333u64;
    pub MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub AdjustmentVariableBlockSize: Multiplier = Multiplier::saturating_from_rational(1, 840);
    pub MinimumMultiplierBlockSize: Multiplier = Multiplier::saturating_from_rational(1, 200u128);
    pub MaximumMultiplierBlockSize: Multiplier = Multiplier::saturating_from_integer(10);

    // TODO: NumberTerms neets to be changed 5
    // when updating to asynchronous backing
    // https://github.com/thrumdev/blobs/issues/166
    // Accepted error is less than 10^(-2)
    pub SkippedBlocksNumberTerms: u32 = 3;
    pub MaximumSkippedBlocks: u32 = sugondat_primitives::MAX_SKIPPED_BLOCKS;

    pub static WeightToFee: u64 = 1;
    pub static OperationalFeeMultiplier: u8 = 5;
    pub static LastRelayBlockNumber: RelayChainBlockNumber = 0;
}

pub struct MockLastRelayBlockNumberProvider;

impl LastRelayBlockNumberProvider for MockLastRelayBlockNumberProvider {
    fn last_relay_block_number() -> RelayChainBlockNumber {
        LastRelayBlockNumber::get()
    }
}

pub fn set_last_relay_block_number(n: RelayChainBlockNumber) {
    LastRelayBlockNumber::mutate(|x| *x = n);
}

impl WeightToFeeT for WeightToFee {
    type Balance = u64;

    fn weight_to_fee(weight: &Weight) -> Self::Balance {
        Self::Balance::saturated_from(weight.ref_time())
            .saturating_mul(WEIGHT_TO_FEE.with(|v| *v.borrow()))
    }
}

impl pallet_transaction_payment::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = WeightToFee;
    type LengthToFee = LengthFeeAdjustment;
    type FeeMultiplierUpdate = ();
}

impl pallet_sugondat_length_fee_adjustment::Config for Test {
    type TransactionByteFee = TransactionByteFee;
    type MaximumBlockLength = MaximumBlockLength;
    type AdjustmentVariableBlockSize = AdjustmentVariableBlockSize;
    type MinimumMultiplierBlockSize = MinimumMultiplierBlockSize;
    type MaximumMultiplierBlockSize = MaximumMultiplierBlockSize;
    type SkippedBlocksNumberTerms = SkippedBlocksNumberTerms;
    type MaximumSkippedBlocks = MaximumSkippedBlocks;
    type LastRelayBlockNumberProvider = MockLastRelayBlockNumberProvider;
}
