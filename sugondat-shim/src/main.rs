mod adapters;
mod cli;
mod cmd;
mod sugondat_rpc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cmd::dispatch().await
}
