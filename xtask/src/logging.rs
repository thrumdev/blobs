use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
};
use tokio::{
    process::{Child, Command as TokioCommand},
    task::JoinHandle,
};
use tracing::warn;

// If log_path is relative it will be made absolute relative to the project_path
//
// The absolute path of where the log file is created is returned
pub fn create_log_file(project_path: &Path, log_path: &String) -> Option<PathBuf> {
    let mut log_path: PathBuf = Path::new(&log_path).to_path_buf();

    if log_path.is_relative() {
        log_path = project_path.join(log_path);
    }

    if let Some(prefix) = log_path.parent() {
        create_dir_all(prefix)
            .map_err(|e| warn!("Impossible to redirect logs, using stdout instead. Error: {e}"))
            .ok()?;
    }

    File::create(&log_path)
        .map_err(|e| warn!("Impossible to redirect logs, using stdout instead. Error: {e}"))
        .ok()?;

    Some(log_path)
}

// These methods will accept a command description and a log path.
//
// The description will be logged at the info log level. The command will be modified and converted to
// a tokio::process::Command, redirecting stdout and stderr.
//
// If the log is None, stdout and stderr will be redirected to the caller's stdout.
// If it contains a path, an attempt will be made to use it to redirect the command's
// stdout and stderr. If, for any reason, the log file cannot be opened or created,
// redirection will default back to the caller's stdout.
pub trait WithLogs {
    fn with_logs(self, description: &str, log_path: &Option<PathBuf>) -> TokioCommand;
    fn spawn_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> anyhow::Result<Child>;
    fn run_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> JoinHandle<anyhow::Result<()>>;
}

impl WithLogs for StdCommand {
    fn with_logs(self, description: &str, log_path: &Option<PathBuf>) -> TokioCommand {
        tracing::info!("{description}");

        let (stdout, stderr) = log_path
            .as_ref()
            .and_then(|log_path| {
                match std::fs::File::options()
                    .append(true)
                    .create(true)
                    .open(log_path.clone())
                {
                    // If log file exists then use it
                    Ok(mut log_out_file) => {
                        let Ok(log_err_file) = log_out_file.try_clone() else {
                            return Some((Stdio::inherit(), Stdio::inherit()));
                        };

                        let _ = log_out_file
                            .write(format!("{}\n", description).as_bytes())
                            .map_err(|e| {
                                warn!("Error writing into {}, error: {e}", log_path.display())
                            });
                        let _ = log_out_file.flush().map_err(|e| {
                            warn!("Error writing into {}, error: {e}", log_path.display())
                        });
                        Some((Stdio::from(log_out_file), Stdio::from(log_err_file)))
                    }
                    // If log file does not exist then use inherited stdout and stderr
                    Err(_) => Some((Stdio::inherit(), Stdio::inherit())),
                }
            })
            // If log file is not specified use inherited stdout and stderr
            .unwrap_or((Stdio::inherit(), Stdio::inherit()));

        let mut command = TokioCommand::from(self);
        command.stderr(stderr).stdout(stdout);
        command
    }

    fn spawn_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> anyhow::Result<tokio::process::Child> {
        self.with_logs(description, log_path)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| e.into())
    }

    fn run_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let description = String::from(description);
        let log_path = log_path.clone();
        tokio::task::spawn(async move {
            let exit_status: anyhow::Result<std::process::ExitStatus> = self
                .spawn_with_logs(&description, &log_path)?
                .wait()
                .await
                .map_err(|e| e.into());
            match exit_status?.code() {
                Some(code) if code != 0 => Err(anyhow::anyhow!(
                    "{description}, exit with status code: {code}",
                )),
                _ => Ok(()),
            }
        })
    }
}

impl<'a> WithLogs for xshell::Cmd<'a> {
    fn with_logs(self, description: &str, log_path: &Option<PathBuf>) -> TokioCommand {
        StdCommand::from(self).with_logs(description, log_path)
    }

    fn spawn_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> anyhow::Result<tokio::process::Child> {
        StdCommand::from(self).spawn_with_logs(description, log_path)
    }

    fn run_with_logs(
        self,
        description: &str,
        log_path: &Option<PathBuf>,
    ) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        StdCommand::from(self).run_with_logs(description, log_path)
    }
}
