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
        /// If the test is executed in CI
        #[clap(long, default_value = "false")]
        pub ci: bool,

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
        #[arg(long = "skip-build", value_name = "skip", id = "build.skip")]
        pub skip: bool,

        /// Build process stdout and stderr are redirected into this file
        #[arg(
            long = "build-log-path",
            value_name = "log-path",
            id = "build.log-path"
        )]
        #[clap(default_value = "test_log/build.log")]
        pub log_path: String,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct ShimParams {
        /// Shim process stdout and stderr are redirected into this file
        #[arg(long = "shim-log-path", value_name = "log-path", id = "shim.log-path")]
        #[clap(default_value = "test_log/shim.log")]
        pub log_path: String,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct ZombienetParams {
        /// Zombienet process stdout and stderr are redirected into this file
        #[arg(
            long = "zombienet-log-path",
            value_name = "log-path",
            id = "zombienet.log-path"
        )]
        #[clap(default_value = "test_log/zombienet.log")]
        pub log_path: String,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct SovereignParams {
        /// Sovereign rollup process stdout and stderr are redirected into this file
        #[arg(
            long = "sovereign-log-path",
            value_name = "log-path",
            id = "sovereign.log-path"
        )]
        #[clap(default_value = "test_log/sovereign.log")]
        pub log_path: String,
    }
}
