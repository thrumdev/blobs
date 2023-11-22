//! Key management: sr25519 account key used for signing blob submission
//! transactions.

use subxt_signer::sr25519::{Seed, Keypair};
use std::path::Path;

/// Load a key from the provided file path.
///
/// The file should contain a hex-encoded 32-byte seed used to generate
/// the underlying schnorrkel private key.
pub fn load_key_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Keypair> {
    let raw = hex::decode(std::fs::read(path)?)?;
    let mut seed: Seed = Seed::default();
    if raw.len() <= seed.len() {
        anyhow::bail!("Keyfile length invalid")
    }
    seed.copy_from_slice(&raw[..]);
    Ok(Keypair::from_seed(seed)?)
}

/// The default dev key.
pub fn alice() -> Keypair {
    subxt_signer::sr25519::dev::alice()
}
