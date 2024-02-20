use crate::{check_binary, cli::test::SovereignParams, logging::create_with_logs};
use anyhow::bail;
use duct::cmd;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct Sovereign {
    process: duct::Handle,
    with_logs: Box<dyn Fn(&str, duct::Expression) -> duct::Expression>,
    project_path: PathBuf,
}

impl Sovereign {
    // Try launching the sovereing rollup using zombienet
    pub fn try_new(project_path: &Path, params: SovereignParams) -> anyhow::Result<Self> {
        info!("Deleting rollup db if it already exists");

        let sovereign_demo_data = project_path.join("demo/sovereign/demo-rollup/demo_data");
        if sovereign_demo_data.as_path().exists() {
            cmd!("rm", "-r", sovereign_demo_data).run()?;
        }

        check_binary(
            "sov-demo-rollup",
            "'sov-demo-rollup' is not found in PATH.  \n \
             cd to 'demo/sovereign/demo-rollup' and run 'cargo build --release' and add the result into your PATH."
        )?;

        info!("Sovereign logs redirected to {}", params.log_path);
        let with_logs = create_with_logs(project_path, params.log_path.clone());

        let sov_demo_rollup_path = project_path.join("demo/sovereign/demo-rollup/");
        //TODO: https://github.com/thrumdev/blobs/issues/227
        #[rustfmt::skip]
        let sovereign_handle = with_logs(
            "Launching sovereign rollup",
            cmd!(
                "sh", "-c",
                format!(
                    "cd {} && sov-demo-rollup",
                    sov_demo_rollup_path.to_string_lossy()
                )
            ),
        ).start()?;

        Ok(Self {
            process: sovereign_handle,
            with_logs,
            project_path: project_path.to_path_buf(),
        })
    }

    // All the networks must be up (relaychain and ikura-node), including the sovereign rollup."
    pub fn test_sovereign_rollup(&self) -> anyhow::Result<()> {
        check_binary(
            "sov-cli",
            "'sov-cli' is not found in PATH.  \n \
             cd to 'demo/sovereign/demo-rollup' and run 'cargo build --release' and add the result into your PATH."
        )?;

        info!("Running sovereign rollup test");

        //TODO: https://github.com/thrumdev/blobs/issues/227
        let sov_demo_rollup_path = self.project_path.join("demo/sovereign/demo-rollup/");
        let test_data_path = "../test-data/";
        let run_cli_cmd =
            |description: &str, args: &str| -> std::io::Result<std::process::Output> {
                let args = [
                    "-c",
                    &format!(
                        "cd {} && sov-cli {args}",
                        sov_demo_rollup_path.to_string_lossy()
                    ),
                ];

                (self.with_logs)(description, duct::cmd("sh", args)).run()
            };

        run_cli_cmd("setup rpc endpoint", "rpc set-url http://127.0.0.1:12345")?;

        run_cli_cmd(
            "import keys",
            &format!("keys import --nickname token_deployer --path {}keys/token_deployer_private_key.json", test_data_path),
        )?;

        run_cli_cmd(
            "create a new token",
            &format!(
                "transactions import from-file bank --path {}requests/create_token.json",
                test_data_path
            ),
        )?;

        run_cli_cmd(
            "mint just created token",
            &format!(
                "transactions import from-file bank --path {}requests/mint.json",
                test_data_path
            ),
        )?;

        run_cli_cmd(
            "submit batch with two transactions",
            "rpc submit-batch by-nickname token_deployer",
        )?;

        // TODO: https://github.com/thrumdev/blobs/issues/226
        info!("waiting for the rollup to process the transactions");
        std::thread::sleep(std::time::Duration::from_secs(30));

        let response = cmd!("sh", "-c", "curl -s -X POST -H \
                              \"Content-Type: application/json\" \
                              -d '{\"jsonrpc\":\"2.0\",\"method\":\"bank_supplyOf\",\
                              \"params\":[\"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72\"],\
                              \"id\":1}' http://127.0.0.1:12345").stdout_capture().run()?;

        if let None = String::from_utf8(response.stdout)?.find("\"amount\":4000") {
            bail!("Tokens not properly minted in the rollup")
        }

        info!("4000 tokens properly minted in the rollup");

        Ok(())
    }
}

impl Drop for Sovereign {
    // duct::Handle does not implement kill on drop
    fn drop(&mut self) {
        info!("Sovereign rollup process is going to be killed");
        let _ = self.process.kill();
    }
}
