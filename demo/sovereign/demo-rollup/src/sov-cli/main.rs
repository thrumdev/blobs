use demo_stf::runtime::RuntimeSubcommand;
use sov_demo_rollup::IkuraDemoRollup;
use sov_modules_api::cli::{FileNameArg, JsonStringArg};
use sov_modules_rollup_template::WalletTemplate;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    IkuraDemoRollup::run_wallet::<
        RuntimeSubcommand<FileNameArg, _, _>,
        RuntimeSubcommand<JsonStringArg, _, _>,
    >()
    .await
}
