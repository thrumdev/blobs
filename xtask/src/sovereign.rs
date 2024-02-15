use crate::{cli::test::SovereignParams, start_maybe_quiet};
use anyhow::bail;
use duct::cmd;
use tracing::info;

pub struct Sovereign(duct::Handle);

impl Sovereign {
    // Try launching the sovereing rollup using zombienet
    pub fn try_new(params: SovereignParams) -> anyhow::Result<Self> {
        info!("Deleting rollup db if it already exists");
        cmd!("rm", "-r", "demo/sovereign/demo-rollup/demo_data")
            .unchecked()
            .run()?;

        //TODO: https://github.com/thrumdev/blobs/issues/227
        info!("Launching sovereign rollup");
        #[rustfmt::skip]
        let sovereign_handle = start_maybe_quiet(
            cmd!(
                "sh", "-c",
                "cd demo/sovereign/demo-rollup && ./../target/release/sov-demo-rollup"
            ),
            params.quiet,
        )?;

        Ok(Self(sovereign_handle))
    }

    // All the networks must be up (relaychain and sugondat-node), including the sovereign rollup."
    pub fn test_sovereign_rollup(&self) -> anyhow::Result<()> {
        info!("Running sovereign rollup test");

        //TODO: https://github.com/thrumdev/blobs/issues/227
        let cli = "../target/release/sov-cli";
        let test_data_path = "../test-data/";
        let run_cli_cmd = |args: &str| {
            let args = [
                "-c",
                &format!("cd demo/sovereign/demo-rollup/ && ./{} {}", cli, args),
            ];

            duct::cmd("sh", args).run()
        };

        info!("setup rpc endpoint");
        run_cli_cmd("rpc set-url http://127.0.0.1:12345")?;

        info!("import keys");
        run_cli_cmd(&format!(
            "keys import --nickname token_deployer --path {}keys/token_deployer_private_key.json",
            test_data_path
        ))?;

        info!("create and mint a new token");
        run_cli_cmd(&format!(
            "transactions import from-file bank --path {}requests/create_token.json",
            test_data_path
        ))?;

        run_cli_cmd(&format!(
            "transactions import from-file bank --path {}requests/mint.json",
            test_data_path
        ))?;

        info!("submit batch with two transactions");
        run_cli_cmd("rpc submit-batch by-nickname token_deployer")?;

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
        let _ = self.0.kill();
    }
}
