use crate::{cli::test::ZombienetParams, logging::create_with_logs};
use duct::cmd;
use tracing::info;

pub struct Zombienet(duct::Handle);

impl Zombienet {
    // Try launching the network using zombienet
    //
    // The binaries for zombienet and polkadot are expected to be in the PATH,
    // while polkadot-execute-worker and polkadot-prepare-worker
    // need to be in the same directory as the polkadot binary.
    pub fn try_new(params: ZombienetParams) -> anyhow::Result<Self> {
        info!("Deleting the zombienet folder if it already exists");
        cmd!("rm", "-r", "zombienet").unchecked().run()?;

        info!("Checking binaries availability");
        let check_binary = |binary: &'static str, error_msg: &'static str| -> anyhow::Result<()> {
            if let Err(_) = cmd!("sh", "-c", format!("command -v {}", binary))
                .stdout_null()
                .run()
            {
                anyhow::bail!(error_msg);
            }
            Ok(())
        };

        check_binary(
            "zombienet",
            "'zombienet' is not found in PATH. Install zombienet. \n \
             Available at https://github.com/paritytech/zombienet",
        )?;

        check_binary(
            "polkadot",
            "'polkadot' is not found in PATH. Install polkadot \n \
              To obtain, refer to https://github.com/paritytech/polkadot-sdk/tree/master/polkadot#polkadot"
        )?;

        check_binary(
            "sugondat-node",
            "'sugondat-node' is not found in PATH.  \n \
             cd to 'sugondat/chain' and run 'cargo build --release' and add the result into your PATH."
        )?;

        tracing::info!("Zombienet logs redirected to {}", params.log_path);
        let with_logs = create_with_logs(params.log_path);

        #[rustfmt::skip]
        let zombienet_handle = with_logs(
            "Launching zombienet",
            cmd!("zombienet", "spawn", "-p", "native", "--dir", "zombienet", "testnet.toml"),
        ).start()?;

        Ok(Self(zombienet_handle))
    }
}

impl Drop for Zombienet {
    // duct::Handle does not implement kill on drop
    fn drop(&mut self) {
        // TODO: https://github.com/thrumdev/blobs/issues/228
        info!("Zombienet process is going to be killed");
        let _ = self.0.kill();
    }
}
