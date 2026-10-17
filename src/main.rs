use std::process::ExitCode;

use replace::cli::Cli;
use replace::error::ReplaceError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            replace::terminal::print_error(&error);
            error.exit_code()
        }
    }
}

fn run() -> Result<(), ReplaceError> {
    Cli::parse_args()?.execute()
}
