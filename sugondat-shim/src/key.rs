//! Key management: sr25519 account key used for signing blob submission
//! transactions.

use std::path::Path;
use subxt_signer::sr25519::Seed;

pub use subxt_signer::sr25519::Keypair;

/// Load a key from the provided file path.
///
/// The file should contain a hex-encoded 32-byte seed used to generate
/// the underlying schnorrkel private key.
pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Keypair> {
    let raw = hex::decode(std::fs::read(path)?)?;
    let mut seed: Seed = Seed::default();
    if raw.len() != seed.len() {
        anyhow::bail!(
            "Keyfile length invalid, expected {} bytes, got {} bytes",
            seed.len(),
            raw.len()
        );
    }
    seed.copy_from_slice(&raw[..]);
    Ok(Keypair::from_seed(seed)?)
}

/// The default dev key.
pub fn alice() -> Keypair {
    subxt_signer::sr25519::dev::alice()
}

#[test]
fn load_alice_key() {
    use std::fs;
    use temp_dir::TempDir;
    // `subkey inspect //Alice`:
    const ALICE_SECRET_SEED_HEX: &str =
        "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a";
    let dir = TempDir::new().unwrap();
    let alice_key = dir.child("alice.key");
    fs::write(&alice_key, ALICE_SECRET_SEED_HEX).unwrap();
    let actual_alice_pubk = load(&alice_key).unwrap().public_key();
    let expected_alice_pubk = alice().public_key();
    assert_eq!(
        hex::encode(actual_alice_pubk.as_ref()),
        hex::encode(expected_alice_pubk.as_ref()),
    );
}
