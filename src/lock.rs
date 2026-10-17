use std::fs::{File, OpenOptions};
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use crate::error::ReplaceError;

/// Acquire an exclusive lock on a file.
///
/// The returned `File` is locked and must be explicitly unlocked when done.
///
/// # Errors
///
/// Returns an error if the lock file cannot be opened or locked.
pub fn lock_file(path: impl AsRef<Path>) -> Result<File, ReplaceError> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    fs4::FileExt::lock(&file)?;
    Ok(file)
}

/// Acquire an exclusive lock on a file with a timeout.
///
/// Prints progress messages to stderr when stderr is a TTY.
///
/// The returned `File` is locked and must be explicitly unlocked when done.
///
/// # Errors
///
/// Returns an error if the file cannot be opened, the lock cannot be acquired within the timeout,
/// or the timeout expires.
pub fn lock_file_with_timeout(
    path: impl AsRef<Path>,
    timeout: Duration,
    lock_name: &str,
) -> Result<File, ReplaceError> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.as_ref())?;

    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);
    let mut last_message_time = std::time::Instant::now();

    loop {
        if matches!(fs4::FileExt::try_lock(&file), Ok(())) {
            return Ok(file);
        }
        if start.elapsed() >= timeout {
            return Err(ReplaceError::LockTimeout(format!(
                "timed out after {:.1}s waiting for {} lock",
                timeout.as_secs_f64(),
                lock_name
            )));
        }

        if is_tty_stderr() && last_message_time.elapsed() >= Duration::from_secs(1) {
            eprint!("\rWaiting for {lock_name} lock...");
            last_message_time = std::time::Instant::now();
        }

        std::thread::sleep(poll_interval);
    }
}

fn is_tty_stderr() -> bool {
    std::io::stderr().is_terminal()
}
