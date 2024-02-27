use crate::{
    check_binary,
    cli::test::ShimParams,
    logging::{create_log_file, WithLogs},
};

pub struct Shim(tokio::process::Child);

impl Shim {
    // Try launching the shim, it requires an up an running ikura-node
    pub async fn try_new(
        project_path: &std::path::Path,
        params: ShimParams,
    ) -> anyhow::Result<Self> {
        check_binary(
            "ikura-shim",
            "'ikura-node' is not found in PATH.  \n \
             cd to 'ikura/shim' and run 'cargo build --release' and add the result into your PATH.",
        )?;

        tracing::info!("Shim logs redirected to {}", params.log_path);
        let log_path = create_log_file(project_path, &params.log_path);

        let sh = xshell::Shell::new()?;

        // Wait for the shim to be connected, which indicates that the network is ready
        xshell::cmd!(sh, "ikura-shim query block --wait 1")
            .run_with_logs("Wait for the network to be ready", &log_path)
            .await??;

        let shim_process = xshell::cmd!(sh, "ikura-shim serve sov --submit-dev-alice")
            .spawn_with_logs("Launching Shim", &log_path)?;

        Ok(Self(shim_process))
    }
}
