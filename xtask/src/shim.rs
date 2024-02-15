use crate::{cli::test::ShimParams, run_maybe_quiet, start_maybe_quiet};
use duct::cmd;
use tracing::info;

pub struct Shim(duct::Handle);

impl Shim {
    // Try launching the shim, it requires an up an running sugondat-node
    pub fn try_new(params: ShimParams) -> anyhow::Result<Self> {
        // Wait for the shim to be connected, which indicates that the network is ready
        info!("Wait for the network to be ready");
        run_maybe_quiet(
            cmd!("sugondat-shim", "query", "block", "--wait", "1",).dir("target/release/"),
            params.quiet,
        )?;

        info!("Launching Shim");
        let shim_handle = start_maybe_quiet(
            cmd!("sugondat-shim", "serve", "--submit-dev-alice").dir("target/release/"),
            params.quiet,
        )?;

        Ok(Self(shim_handle))
    }
}

impl Drop for Shim {
    // duct::Handle does not implement kill on drop
    fn drop(&mut self) {
        info!("Shim process is going to be killed");
        let _ = self.0.kill();
    }
}
