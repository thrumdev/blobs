use crate::command::new_partial;
use sc_client_api::HeaderBackend;
use sc_service::Configuration;
use sp_api::{Metadata, ProvideRuntimeApi};
use sp_core::hexdisplay::HexDisplay;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Export the genesis metadata of the parachain.
///
/// This will print the metadata into the specified file or stdout if no file is specified. By
/// default, the metadata is printed in hex format. Use the `--raw` flag to print the metadata in
/// binary format.
///
/// To use it with subxt, you can use the following commands:
///
///     $ sugondat-node export-genesis-metadata --raw > metadata.bin
///     $ subxt codegen --file metadata.bin | rustfmt --edition=2021 --emit=stdout > src/metadata.rs
///
#[derive(Debug, clap::Parser)]
#[clap(verbatim_doc_comment)]
pub struct ExportGenesisMetadataCmd {
    #[allow(missing_docs)]
    #[command(flatten)]
    pub shared_params: sc_cli::SharedParams,

    /// Output file name or stdout if unspecified.
    #[arg()]
    pub output: Option<PathBuf>,

    /// Write output in binary. Default is to write in hex.
    #[arg(short, long)]
    pub raw: bool,
}

impl ExportGenesisMetadataCmd {
    /// Exports the metadata for the genesis block.
    ///
    /// Basically, this returns the metadata returned from the compiled-in runtime.
    pub fn run(&self, config: &Configuration) -> sc_cli::Result<()> {
        let partials = new_partial(&config)?;
        let client = partials.client.clone();
        let hash = client.info().genesis_hash;
        let metadata: sp_core::OpaqueMetadata = client
            .runtime_api()
            .metadata(hash)
            .map_err(|e| format!("Failed to fetch metadata from client: {:?}", e))?;

        let output_buf: Vec<u8> = if self.raw {
            metadata.to_vec()
        } else {
            format!("0x{:?}", HexDisplay::from(&*metadata)).into_bytes()
        };

        if let Some(output) = &self.output {
            fs::write(output, output_buf)?;
        } else {
            io::stdout().write_all(&output_buf)?;
        }

        Ok(())
    }
}

impl sc_cli::CliConfiguration for ExportGenesisMetadataCmd {
    fn shared_params(&self) -> &sc_cli::SharedParams {
        &self.shared_params
    }
}
