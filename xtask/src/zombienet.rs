use crate::{
    check_binary,
    cli::ZombienetParams,
    logging::{create_log_file, WithLogs},
    ProcessGroupId,
};
use std::path::Path;
use tracing::info;
use xshell::cmd;

pub struct Zombienet(tokio::process::Child);

impl Zombienet {
    // Try launching the network using zombienet
    //
    // The binaries for zombienet and polkadot are expected to be in the PATH,
    // while polkadot-execute-worker and polkadot-prepare-worker
    // need to be in the same directory as the polkadot binary.
    pub async fn try_new(project_path: &Path, params: ZombienetParams) -> anyhow::Result<Self> {
        let sh = xshell::Shell::new()?;

        info!("Deleting the zombienet folder if it already exists");
        let zombienet_folder = project_path.join("zombienet");
        if zombienet_folder.as_path().exists() {
            cmd!(sh, "rm -r {zombienet_folder}")
                .quiet()
                .ignore_status()
                .run()?;
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
        let log_path = create_log_file(project_path, &params.log_path);

        sh.change_dir(project_path);

        let zombienet_process = cmd!(sh, "zombienet spawn -p native --dir zombienet testnet.toml")
            .process_group(0)
            .spawn_with_logs("Launching zombienet", &log_path)?;

        Ok(Self(zombienet_process))
    }
}

impl Drop for Zombienet {
    fn drop(&mut self) {
        use nix::{sys::signal, unistd::Pid};
        let Some(id) = self.0.id() else { return };
        signal::killpg(Pid::from_raw(id as i32), Some(signal::Signal::SIGKILL))
            .expect("Failed kill zombienet process");
    }
}
