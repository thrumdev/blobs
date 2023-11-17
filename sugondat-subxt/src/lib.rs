use subxt::OnlineClient;

mod gen;

pub mod sugondat {
	pub use super::gen::api::*;
}

pub type SugondatConfig = subxt::SubstrateConfig;

pub type Client = OnlineClient<SugondatConfig>;

pub type Header = <SugondatConfig as subxt::Config>::Header;

// #[derive(Clone, Debug, Default)]
// pub struct SugondatConfig;

// impl subxt::Config for SugondatConfig {
//     type AccountId = sugondat_runtime::AccountId;
// 	type Address = sugondat_runtime::Address;
// 	// type ExtrinsicParams = sugondat_runtime::AvailExtrinsicParams;
// 	type Hash = sugondat_runtime::H256;
// 	type Hasher = sp_runtime::traits::BlakeTwo256;
// 	type Header = sugondat_runtime::Header;
// 	type Index = sugondat_runtime::Index;
// 	type Signature = sugondat_runtime::Signature;
// }

/// Creates a client and validate the code generation if `validate_codegen == true`.
pub async fn build_client<U: AsRef<str>>(
	url: impl AsRef<str>,
	validate_codegen: bool,
) -> anyhow::Result<Client> {
	let api = Client::from_url(url).await?;
	if validate_codegen && !sugondat::is_codegen_valid_for(&api.metadata()) {
		anyhow::bail!("Client metadata not valid")
	}
	Ok(api)
}
