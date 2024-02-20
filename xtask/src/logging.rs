use std::path::Path;
use std::{io::Write, path::PathBuf};
use tracing::{info, warn};

// If log_path is relative it will be made absolute relative to the project_path
//
// The absolute path of where the log file is created is returned
fn create_log_file(project_path: &Path, log_path: &String) -> std::io::Result<PathBuf> {
    let mut log_path: PathBuf = Path::new(&log_path).to_path_buf();

    if log_path.is_relative() {
        log_path = project_path.join(log_path);
    }

    if let Some(prefix) = log_path.parent() {
        std::fs::create_dir_all(prefix)?;
    }
    std::fs::File::create(&log_path)?;
    Ok(log_path)
}

// If the log file cannot be created due to any reasons,
// such as lack of permission to create files or new folders in the path,
// things will be printed to stdout instead of being redirected to the logs file
//
// The returned closure will accept a description of the command and the command itself as a duct::Expression.
// The description will be printed to both stdout and the log file, if possible, while
// to the expression will be added the redirection of the logs, if possible.
pub fn create_with_logs(
    project_path: &Path,
    log_path: String,
) -> Box<dyn Fn(&str, duct::Expression) -> duct::Expression> {
    let without_logs = |description: &str, cmd: duct::Expression| -> duct::Expression {
        info!("{description}");
        cmd
    };

    let log_path = match create_log_file(project_path, &log_path) {
        Ok(log_path) => log_path,
        Err(e) => {
            warn!("Impossible redirect logs, using stdout instead. Error: {e}");
            return Box::new(without_logs);
        }
    };

    let with_logs = move |description: &str, cmd: duct::Expression| -> duct::Expression {
        // The file has just been created
        let mut log_file = std::fs::File::options()
            .append(true)
            .open(&log_path)
            .unwrap();

        info!("{description}");
        let log_path = log_path.to_string_lossy();
        let _ = log_file
            .write(format!("{}\n", description).as_bytes())
            .map_err(|e| warn!("Error writing into {log_path}, error: {e}",));
        let _ = log_file
            .flush()
            .map_err(|e| warn!("Error writing into {log_path}, error: {e}",));
        cmd.stderr_to_stdout().stdout_file(log_file)
    };

    Box::new(with_logs)
}
