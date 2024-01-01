mod cli;
mod cmd;
mod dock;
mod key;
mod sugondat_rpc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cmd::dispatch().await
}
