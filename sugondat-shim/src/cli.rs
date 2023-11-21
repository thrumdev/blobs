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

/// Common parameters for the adapter subcommands.
#[derive(clap::Args, Debug)]
pub struct AdapterServerParams {
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
    // TODO: e.g. --submit-key, prometheus stuff, enabled adapters, etc.
}

/// Common parameters for that commands that connect to the sugondat-node.
#[derive(clap::Args, Debug)]
pub struct SugondatRpcParams {
    /// The address of the sugondat-node to connect to.
    #[clap(long, default_value = "ws://localhost:9944", env = ENV_SUGONDAT_NODE_URL)]
    pub node_url: String,
}

impl AdapterServerParams {
    /// Whether the sovereign adapter should be enabled.
    pub fn enable_sovereign(&self) -> bool {
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

    use super::{AdapterServerParams, SugondatRpcParams};
    use clap::Args;

    #[derive(Debug, Args)]
    pub struct Params {
        #[clap(flatten)]
        pub rpc: SugondatRpcParams,

        #[clap(flatten)]
        pub adapter: AdapterServerParams,
    }
}

pub mod query {
    //! CLI definition for the `query` subcommand.

    // TODO: I envision several subcommands here. For example:
    // - query block <block_hash/number> — returns the information about a block and header.
    // - query blob <id> - returns the blob for a given key. The key here is the same sense as
    //   described here https://github.com/thrumdev/sugondat/issues/9#issuecomment-1814005570.

    use super::{SugondatRpcParams, ENV_SUGONDAT_NAMESPACE};
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
    }

    pub mod submit {
        //! CLI definition for the `query submit` subcommand.

        use super::{SugondatRpcParams, ENV_SUGONDAT_NAMESPACE};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Params {
            #[clap(flatten)]
            pub rpc: SugondatRpcParams,

            /// The namespace to submit the blob into.
            ///
            /// The namespace can be specified either as a 4-byte vector, or as an unsigned 32-bit
            /// integer. To distinguish between the two, the byte vector must be prefixed with
            /// `0x`.
            #[clap(long, short, env = ENV_SUGONDAT_NAMESPACE)]
            pub namespace: String,

            /// The file path of the blob to submit. Pass `-` to read from stdin.
            pub blob_path: String,
        }
    }
}
