use std::{
    fmt::Display,
    io::{self, Write},
};

use owo_colors::OwoColorize;

pub fn print_error(error: impl Display) {
    if writeln!(io::stderr().lock(), "{} {error}", "error:".red().bold()).is_err() {}
}
