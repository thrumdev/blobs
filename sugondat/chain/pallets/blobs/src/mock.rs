use crate as pallet_blobs;
use frame_support::{parameter_types, traits::ConstU32};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
    BuildStorage, MultiSignature,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Real accounts must be used to test and benchmark on_finalize
pub type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Blobs: crate::{Pallet, Call, Storage, Event<T>},
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
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_blobs::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxBlobs = ConstU32<16>;
    type MaxBlobSize = ConstU32<1024>;
    type MaxTotalBlobSize = ConstU32<{ 10 * 1024 }>;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
// TODO: https://github.com/thrumdev/blobs/issues/28
#[allow(unused)]
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}
