use crate::{check_binary, cli::ZombienetParams, logging::create_with_logs};
use duct::cmd;
use std::path::Path;
use tracing::info;

pub struct Zombienet(duct::Handle);

impl Zombienet {
    // Try launching the network using zombienet
    //
    // The binaries for zombienet and polkadot are expected to be in the PATH,
    // while polkadot-execute-worker and polkadot-prepare-worker
    // need to be in the same directory as the polkadot binary.
    pub fn try_new(project_path: &Path, params: ZombienetParams) -> anyhow::Result<Self> {
        info!("Deleting the zombienet folder if it already exists");
        let zombienet_folder = project_path.join("zombienet");
        if zombienet_folder.as_path().exists() {
            cmd!("rm", "-r", zombienet_folder).run()?;
        }

        info!("Checking binaries availability");
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
            "ikura-node",
            "'ikura-node' is not found in PATH.  \n \
             cd to 'ikura/chain' and run 'cargo build --release' and add the result into your PATH."
        )?;

        tracing::info!("Zombienet logs redirected to {}", params.log_path);
        let with_logs = create_with_logs(project_path, params.log_path);

        #[rustfmt::skip]
        let zombienet_handle = with_logs(
            "Launching zombienet",
            cmd!(
                "sh", "-c",
                format!("cd {} && zombienet spawn -p native --dir zombienet testnet.toml", project_path.to_string_lossy())
            ),
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
