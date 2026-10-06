use clap::Parser;
use std::{
    io::{self, Write},
    process::ExitCode,
};
mod args;
mod commands;
mod error;
mod execution_args;
mod output;
use args::Cli;
use error::CliError;
fn main() -> ExitCode {
    let result = commands::run(Cli::parse()).and_then(|text| {
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
        Ok(())
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "error: {error}");
            ExitCode::FAILURE
        }
    }
}
