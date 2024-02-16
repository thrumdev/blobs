use crate::{cli::test::ShimParams, logging::create_with_logs};
use duct::cmd;
use tracing::info;

pub struct Shim(duct::Handle);

impl Shim {
    // Try launching the shim, it requires an up an running sugondat-node
    pub fn try_new(params: ShimParams) -> anyhow::Result<Self> {
        tracing::info!("Shim logs redirected to {}", params.log_path);
        let with_logs = create_with_logs(params.log_path);

        // Wait for the shim to be connected, which indicates that the network is ready
        with_logs(
            "Wait for the network to be ready",
            cmd!("sugondat-shim", "query", "block", "--wait", "1"),
        )
        .run()?;

        let shim_handle = with_logs(
            "Launching Shim",
            cmd!("sugondat-shim", "serve", "--submit-dev-alice"),
        )
        .start()?;

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
