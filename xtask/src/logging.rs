use std::io::Write;
use std::path::Path;
use tracing::{info, warn};

fn create_log_file(log_path: &String) -> std::io::Result<()> {
    if let Some(prefix) = Path::new(&log_path).parent() {
        std::fs::create_dir_all(prefix)?;
    }
    std::fs::File::create(&log_path)?;
    Ok(())
}

// If the log file cannot be created due to any reasons,
// such as lack of permission to create files or new folders in the path,
// things will be printed to stdout instead of being redirected to the logs file
//
// The returned closure will accept a description of the command and the command itself as a duct::Expression.
// The description will be printed to both stdout and the log file, if possible, while
// to the expression will be added the redirection of the logs, if possible.
pub fn create_with_logs(
    log_path: String,
) -> Box<dyn Fn(&str, duct::Expression) -> duct::Expression> {
    let without_logs = |description: &str, cmd: duct::Expression| -> duct::Expression {
        info!("{description}");
        cmd
    };

    if let Err(e) = create_log_file(&log_path) {
        warn!("Impossible redirect to {log_path}, using stdout instead. Error: {e}");
        return Box::new(without_logs);
    }

    let with_logs = move |description: &str, cmd: duct::Expression| -> duct::Expression {
        // The file has just been created
        let mut log_file = std::fs::File::options()
            .append(true)
            .open(&log_path)
            .unwrap();

        info!("{description}");
        let _ = log_file
            .write(format!("{}\n", description).as_bytes())
            .map_err(|e| warn!("Error writing into {log_path}, error: {e}"));
        let _ = log_file
            .flush()
            .map_err(|e| warn!("Error writing into {log_path}, error: {e}"));
        cmd.stderr_to_stdout().stdout_file(log_file)
    };

    Box::new(with_logs)
}
