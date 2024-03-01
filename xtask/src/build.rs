use crate::{
    cli::BuildParams,
    logging::{create_log_file, WithLogs},
};
use xshell::cmd;

// TODO: https://github.com/thrumdev/blobs/issues/225

pub async fn build(project_path: &std::path::Path, params: BuildParams) -> anyhow::Result<()> {
    if params.skip {
        return Ok(());
    }

    let sh = xshell::Shell::new()?;

    tracing::info!("Building logs redirected {}", params.log_path);
    let log_path = create_log_file(project_path, &params.log_path);

    // `it is advisable to use CARGO environmental variable to get the right cargo`
    // quoted by xtask readme
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    cmd!(sh, "{cargo} build -p ikura-node --release")
        .run_with_logs("Building ikura-node", &log_path)
        .await??;

    cmd!(sh, "{cargo} build -p ikura-shim --release")
        .run_with_logs("Building ikura-shim", &log_path)
        .await??;

    sh.change_dir(project_path.join("demo/sovereign/demo-rollup/"));
    cmd!(sh, "{cargo} build --release")
        .run_with_logs("Building sovereign demo-rollup", &log_path)
        .await??;

    Ok(())
}
