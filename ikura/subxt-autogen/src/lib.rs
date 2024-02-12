use subxt::OnlineClient;

mod gen;

pub mod ikura {
    pub use super::gen::api::*;
}

pub type IkuraConfig = subxt::SubstrateConfig;

pub type Client = OnlineClient<IkuraConfig>;

pub type Header = <IkuraConfig as subxt::Config>::Header;

pub type ExtrinsicDetails = subxt::blocks::ExtrinsicDetails<IkuraConfig, Client>;

// #[derive(Clone, Debug, Default)]
// pub struct IkuraConfig;

// impl subxt::Config for IkuraConfig {
//     type AccountId = ikura_runtime::AccountId;
// 	type Address = ikura_runtime::Address;
// 	// type ExtrinsicParams = ikura_runtime::AvailExtrinsicParams;
// 	type Hash = ikura_runtime::H256;
// 	type Hasher = sp_runtime::traits::BlakeTwo256;
// 	type Header = ikura_runtime::Header;
// 	type Index = ikura_runtime::Index;
// 	type Signature = ikura_runtime::Signature;
// }

/// Creates a client and validate the code generation if `validate_codegen == true`.
pub async fn build_client<U: AsRef<str>>(
    url: impl AsRef<str>,
    validate_codegen: bool,
) -> anyhow::Result<Client> {
    let api = Client::from_url(url).await?;
    if validate_codegen && !ikura::is_codegen_valid_for(&api.metadata()) {
        anyhow::bail!("Client metadata not valid")
    }
    Ok(api)
}
