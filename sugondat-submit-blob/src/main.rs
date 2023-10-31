use std::env;
use sugondat_da_adapter::service::DaProvider;
use sugondat_da_adapter::spec::ChainParams;
use sugondat_da_adapter::service::DaServiceConfig;
use const_rollup_config::ROLLUP_NAMESPACE_RAW;
use sov_rollup_interface::services::da::DaService;

#[tokio::main]
async fn main() {
    let filename = env::args().nth(1).expect("no filename given");
    let data = std::fs::read_to_string(filename).expect("could not read file");

    let da_service = DaProvider::new(
        DaServiceConfig {
            sugondat_rpc: "ws://localhost:9988/".into(),
        },
        ChainParams {
            namespace_id: ROLLUP_NAMESPACE_RAW,
        },
    );

    da_service.send_transaction(data.as_bytes()).await.unwrap();
}
