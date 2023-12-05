use crate::kusama::runtime_types::polkadot_parachain_primitives::primitives::*;
use std::include_bytes;
use subxt::{
    tx::Signer,
    utils::{AccountId32, MultiAddress},
    OnlineClient, PolkadotConfig,
};

#[subxt::subxt(runtime_metadata_path = "kusama_metadata.scale")]
pub mod kusama {}

#[tokio::main]
pub async fn main() {
    // Register the Parachain
    register_parachain().await;
}

async fn register_parachain() {
    let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:8000")
        .await
        .unwrap();
    println!("Connection with kusama fork established.");

    // let alice: MultiAddress<AccountId32, ()> = dev::alice().public_key().into();
    let alice = subxt_signer::sr25519::dev::alice();

    let statefile = include_bytes!("../statefile");
    let wasmfile = include_bytes!("../wasmfile");

    // /* primitives::Id */, /* HeadData */, /* primitives::ValidationCode */
    let register = kusama::tx().registrar().register(
        Id(3338),
        HeadData(statefile.to_vec()),
        ValidationCode(wasmfile.to_vec()),
    );

    let signed = api
        .tx()
        .create_signed(&register, &alice, Default::default())
        .await
        .unwrap();

    //let _ = api
    //    .tx()
    //    .sign_and_submit_then_watch_default(&register, &alice)
    //    .await
    //    .map(|e| {
    //        println!("Collection creation submitted, waiting for transaction to be finalized...");
    //        e
    //    })
    //    .unwrap()
    //    .wait_for_finalized_success()
    //    .await
    //    .unwrap();

    todo!()
}
