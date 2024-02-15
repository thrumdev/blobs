use crate::{cli::test::BuildParams, logging::create_with_logs};
use duct::cmd;

// TODO: https://github.com/thrumdev/blobs/issues/225

pub fn build(params: BuildParams) -> anyhow::Result<()> {
    if params.skip {
        return Ok(());
    }

    tracing::info!("Building logs redirected {}", params.log_path);
    let with_logs = create_with_logs(params.log_path);

    // `it is advisable to use CARGO environmental variable to get the right cargo`
    // quoted by xtask readme
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    with_logs(
        "Building ikura-node",
        cmd!(&cargo, "build", "-p", "ikura-node", "--release"),
    )
    .run()?;

    with_logs(
        "Building ikura-shim",
        cmd!(&cargo, "build", "-p", "ikura-shim", "--release"),
    )
    .run()?;

    #[rustfmt::skip]
    with_logs(
        "Building sovereign demo-rollup",
        cmd!(
            "sh", "-c",
            format!(
                "cd demo/sovereign/demo-rollup/ && {} build --release",
                cargo
            )
        ),
    ).run()?;

    Ok(())
}
