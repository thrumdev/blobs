use crate::{
    check_binary,
    cli::test::SovereignParams,
    logging::{create_log_file, WithLogs},
};
use anyhow::bail;
use std::path::{Path, PathBuf};
use tracing::info;
use xshell::cmd;

pub struct Sovereign {
    pub rollup_process: tokio::process::Child,
    log_path: Option<PathBuf>,
    project_path: PathBuf,
}

impl Sovereign {
    // Try launching the sovereing rollup using zombienet
    pub async fn try_new(project_path: &Path, params: SovereignParams) -> anyhow::Result<Self> {
        let sh = xshell::Shell::new()?;

        info!("Deleting rollup db if it already exists");
        let sovereign_demo_data = project_path.join("demo/sovereign/demo-rollup/demo_data");
        if sovereign_demo_data.as_path().exists() {
            cmd!(sh, "rm -r {sovereign_demo_data}")
                .quiet()
                .ignore_status()
                .run()?;
        }

        check_binary(
            "sov-demo-rollup",
            "'sov-demo-rollup' is not found in PATH.  \n \
             cd to 'demo/sovereign/demo-rollup' and run 'cargo build --release' and add the result into your PATH."
        )?;

        info!("Sovereign logs redirected to {}", params.log_path);
        let log_path = create_log_file(project_path, &params.log_path);

        //TODO: https://github.com/thrumdev/blobs/issues/227
        let sov_demo_rollup_path = project_path.join("demo/sovereign/demo-rollup/");
        sh.change_dir(sov_demo_rollup_path);
        let rollup_process =
            cmd!(sh, "sov-demo-rollup").spawn_with_logs("Launching sovereign rollup", &log_path)?;

        Ok(Self {
            rollup_process,
            log_path,
            project_path: project_path.to_path_buf(),
        })
    }

    // All the networks must be up (relaychain and ikura-node), including the sovereign rollup."
    pub async fn test_sovereign_rollup(&self) -> anyhow::Result<()> {
        check_binary(
            "sov-cli",
            "'sov-cli' is not found in PATH.  \n \
             cd to 'demo/sovereign/demo-rollup' and run 'cargo build --release' and add the result into your PATH."
        )?;

        info!("Running sovereign rollup test");

        //TODO: https://github.com/thrumdev/blobs/issues/227
        let sh = xshell::Shell::new()?;
        let sov_demo_rollup_path = self.project_path.join("demo/sovereign/demo-rollup/");
        sh.change_dir(sov_demo_rollup_path);

        let test_data_path = "../test-data/";

        cmd!(sh, "sov-cli rpc set-url http://127.0.0.1:12345")
            .run_with_logs("setup rpc endpoint", &self.log_path)
            .await??;

        cmd!(sh, "sov-cli keys import --nickname token_deployer --path {test_data_path}keys/token_deployer_private_key.json")
            .run_with_logs("import keys", &self.log_path)
            .await??;

        cmd!(sh, "sov-cli transactions import from-file bank --path {test_data_path}requests/create_token.json")
            .run_with_logs("create a new token", &self.log_path)
            .await??;

        cmd!(
            sh,
            "sov-cli transactions import from-file bank --path {test_data_path}requests/mint.json"
        )
        .run_with_logs("mint just created token", &self.log_path)
        .await??;

        cmd!(sh, "sov-cli rpc submit-batch by-nickname token_deployer")
            .run_with_logs("submit batch with two transactions", &self.log_path)
            .await??;

        // TODO: https://github.com/thrumdev/blobs/issues/226
        info!("waiting for the rollup to process the transactions");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let response = cmd!(sh, "sh -c")
            .arg(
                "curl -s -X POST -H \
                  \"Content-Type: application/json\" \
                  -d '{\"jsonrpc\":\"2.0\",\"method\":\"bank_supplyOf\",\
                  \"params\":[\"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72\"],\
                  \"id\":1}' http://127.0.0.1:12345",
            )
            .quiet()
            .read()?;

        if let None = response.find("\"amount\":4000") {
            bail!("Tokens not properly minted in the rollup")
        }

        info!("4000 tokens properly minted in the rollup");

        Ok(())
    }
}
