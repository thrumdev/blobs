use clap::{Parser, Subcommand};

// NOTE:
//
// The architecture of the CLI may seem contrived, but here are some reasons for it:
//
// - We want to push the parameters into the subcommands, instead of having them on the more general
//   structs. Specifially, we want to avoid
//
//     sugondat-shim -p 10 serve --node-url=...
//
//   because the user will have to remember where each flag must be (e.g. here -p before the
//   subcommand, but --node-url after the subcommand). Besides, it also looks clunky.
//
// - We want to have the CLI definition not to be scatered all over the codebase. Therefore it is
//   defined in a single file.
//
// - We use modules to group the CLI definitions for each subcommand, instead of prefixing and
//   dealing with lots of types like `ServeParams`, `QueryParams`, `QuerySubmitParams`, etc.
//
//   This approach is more verbose, but it is also more explicit and easier to understand.
//   Verbosiness is OK here, because we reserve the entire file for the CLI definitions
//   anyway.
//
// When adding a new subcommand or parameter, try to follow the same patterns as the existing
// ones. Ensure that the flags are consistent with the other subcommands, that the help
// messages are present and clear, etc.

const ENV_SUGONDAT_SHIM_PORT: &str = "SUGONDAT_SHIM_PORT";
const ENV_SUGONDAT_NAMESPACE: &str = "SUGONDAT_NAMESPACE";
const ENV_SUGONDAT_NODE_URL: &str = "SUGONDAT_NODE_URL";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Common parameters for key management in a subcommand.
// TODO: for docks, this should not be required and for query submit it should
// be. Unfortunately, clap doesn't support this easily so it is handled manually
// within the command execution for submit.
#[derive(clap::Args, Debug)]
#[group(multiple = false)]
pub struct KeyManagementParams {
    /// Use the Alice development key to sign blob transactions.
    ///
    /// This key is enabled when running sugondat-node in the local development mode.
    ///
    /// Cannot be used in conjunction with the `--submit-private-key` flag.
    #[arg(long)]
    pub submit_dev_alice: bool,

    /// Use the keyfile at the provided path to sign blob transactions.
    ///
    /// The keyfile should be 32 bytes of unencrypted, hex-encoded sr25519
    /// seed material.
    ///
    /// Cannot be used in conjunction with the `--submit-dev-alice` flag.
    #[arg(long, value_name = "PATH")]
    pub submit_private_key: Option<std::path::PathBuf>,
}

/// Common parameters for the subcommands that run docks.
#[derive(clap::Args, Debug)]
pub struct DockParams {
    /// The address on which the shim should listen for incoming connections from the rollup nodes.
    #[clap(short, long, default_value = "127.0.0.1", group = "listen")]
    pub address: String,

    /// The port on which the shim should listen for incoming connections from the rollup nodes.
    #[clap(
        short,
        long,
        env = ENV_SUGONDAT_SHIM_PORT,
        default_value = "10995",
        group = "listen"
    )]
    pub port: u16,
    // TODO: e.g. prometheus stuff, enabled docks, etc.
}

/// Common parameters for that commands that connect to the sugondat-node.
#[derive(clap::Args, Debug)]
pub struct SugondatRpcParams {
    /// The address of the sugondat-node to connect to.
    #[clap(long, default_value = "ws://localhost:9988", env = ENV_SUGONDAT_NODE_URL)]
    pub node_url: String,
}

impl DockParams {
    /// Whether the sovereign dock should be enabled.
    pub fn enable_sovereign(&self) -> bool {
        true
    }

    /// Whether the rollkit dock should be enabled.
    pub fn enable_rollkit(&self) -> bool {
        true
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Connect to the sugondat node and serve requests from the rollup nodes.
    Serve(serve::Params),
    /// Serve requests from the rollup nodes by simulating the DA layer.
    Simulate,
    /// Allows running queries locally. Useful for debugging.
    Query(query::Params),
}

pub mod serve {
    //! CLI definition for the `serve` subcommand.

    use super::{DockParams, KeyManagementParams, SugondatRpcParams};
    use clap::Args;

    #[derive(Debug, Args)]
    pub struct Params {
        #[clap(flatten)]
        pub rpc: SugondatRpcParams,

        #[clap(flatten)]
        pub dock: DockParams,

        #[clap(flatten)]
        pub key_management: KeyManagementParams,
    }
}

pub mod query {
    //! CLI definition for the `query` subcommand.

    use super::{KeyManagementParams, SugondatRpcParams, ENV_SUGONDAT_NAMESPACE};
    use clap::{Args, Subcommand};

    #[derive(Debug, Args)]
    pub struct Params {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Submits the given blob into a namespace.
        Submit(submit::Params),
        /// Queries information about a block and header.
        ///
        /// Returns an error if the given block is not available.
        Block(block::Params),
        /// Queries information about a specific blob.
        Blob(blob::Params),
    }

    /// A reference to a block to query.
    #[derive(Debug, Clone)]
    pub enum BlockRef {
        /// The current best finalized block known by the node.
        Best,
        /// The number of the block to query.
        Number(u64),
        /// The hex-encoded hash of the block to query, prefixed with "0x".
        Hash([u8; 32]),
    }

    impl std::fmt::Display for BlockRef {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match *self {
                BlockRef::Best => write!(f, "best"),
                BlockRef::Number(n) => write!(f, "{}", n),
                BlockRef::Hash(h) => write!(f, "0x{}", hex::encode(&h[..])),
            }
        }
    }

    impl std::str::FromStr for BlockRef {
        type Err = String;

        fn from_str(input: &str) -> Result<Self, Self::Err> {
            if input == "best" {
                return Ok(BlockRef::Best);
            }

            if let Some(hash) = decode_hash(input)? {
                return Ok(BlockRef::Hash(hash));
            }

            if let Ok(n) = input.parse::<u64>() {
                Ok(BlockRef::Number(n))
            } else {
                Err(format!("parse error. see `--help`"))
            }
        }
    }

    fn decode_hash(input: &str) -> Result<Option<[u8; 32]>, String> {
        if let Some(s) = input.strip_prefix("0x") {
            let bytes =
                hex::decode(s).map_err(|_| "Invalid parameter: not hex encoded".to_owned())?;

            let mut hash = [0u8; 32];
            if bytes.len() != 32 {
                return Err("Invalid parameter: hash not 32 bytes".to_owned());
            }

            hash.copy_from_slice(&bytes[..]);
            Ok(Some(hash))
        } else {
            Ok(None)
        }
    }

    pub mod blob {
        use clap::Args;

        use super::{BlockRef, SugondatRpcParams};

        #[derive(Debug, Args)]
        pub struct Params {
            #[clap(flatten)]
            pub rpc: SugondatRpcParams,

            /// The block containing the blob to query.
            ///
            /// Possible values: ["best", number, hash]
            ///
            /// "best" is the highest finalized block.
            ///
            /// Hashes must be 32 bytes, hex-encoded, and prefixed with "0x".
            #[arg(value_name = "BLOCK_REF")]
            pub block: BlockRef,

            /// The index of the extrinsic (transaction) containing the blob.
            #[arg(value_name = "INDEX")]
            pub index: u32,

            /// Output the blob data as binary to stdout rather than hex, and omits
            /// any other details intended for human consumption.
            #[arg(long)]
            pub raw: bool,
        }
    }

    pub mod block {
        //! CLI definition for the `query block` subcommand.

        use clap::Args;

        use super::{BlockRef, SugondatRpcParams};

        #[derive(Debug, Args)]
        pub struct Params {
            #[clap(flatten)]
            pub rpc: SugondatRpcParams,

            /// The block to query information about.
            ///
            /// Possible values: ["best", number, hash]
            ///
            /// "best" is the highest finalized block.
            ///
            /// Hashes must be 32 bytes, hex-encoded, and prefixed with "0x".
            #[arg(default_value_t = BlockRef::Best, value_name = "BLOCK_REF")]
            pub block: BlockRef,
        }
    }

    pub mod submit {
        //! CLI definition for the `query submit` subcommand.

        use super::{KeyManagementParams, SugondatRpcParams, ENV_SUGONDAT_NAMESPACE};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Params {
            #[clap(flatten)]
            pub rpc: SugondatRpcParams,

            /// The namespace to submit the blob into.
            ///
            /// The namespace can be specified either as a 16-byte vector, or as an unsigned 128-bit
            /// big-endian integer. To distinguish between the two, the byte vector must be prefixed
            ///  with `0x`.
            #[clap(long, short, env = ENV_SUGONDAT_NAMESPACE)]
            pub namespace: String,

            /// The file path of the blob to submit. Pass `-` to read from stdin.
            pub blob_path: String,

            #[clap(flatten)]
            pub key_management: KeyManagementParams,
        }
    }
}
