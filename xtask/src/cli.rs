use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Test(test::Params),
}

pub mod test {

    // TODO: https://github.com/thrumdev/blobs/issues/224
    use clap::Args;
    #[derive(Debug, Args)]
    pub struct Params {
        #[clap(flatten)]
        pub build: BuildParams,

        #[clap(flatten)]
        pub shim: ShimParams,

        #[clap(flatten)]
        pub zombienet: ZombienetParams,

        #[clap(flatten)]
        pub sovereign: SovereignParams,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct BuildParams {
        /// Skip building required binaries
        /// (sugondat-node, sugondat-shim, sov-demo-rollup and sov-cli)
        #[clap(default_value = "false")]
        #[arg(long = "build-skip", value_name = "skip", id = "build.skip")]
        pub skip: bool,

        /// Don't print to stdout during the build process
        #[arg(long = "build-quiet", value_name = "quiet", id = "build.quiet")]
        pub quiet: bool,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct ShimParams {
        /// Don't print shim process stdout
        #[arg(long = "shim-quiet", value_name = "quiet", id = "shim.quiet")]
        pub quiet: bool,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct ZombienetParams {
        /// Don't print zombienet process stdout
        #[arg(long = "zombienet-quiet", value_name = "quiet", id = "zombienet.quiet")]
        pub quiet: bool,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct SovereignParams {
        /// Don't print sovereing rollup processes stdout
        #[arg(long = "sovereign-quiet", value_name = "quiet", id = "sovereign.quiet")]
        pub quiet: bool,
    }
}
