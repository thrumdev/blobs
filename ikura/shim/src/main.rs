mod cli;
mod cmd;
mod dock;
mod ikura_rpc;
mod key;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cmd::dispatch().await
}
