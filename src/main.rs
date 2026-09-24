mod cli;
mod commands;
mod config;
mod error;
mod fsutil;
mod http;
mod media;
mod nfo;
mod plan;
mod tmdb;

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    match commands::run(cli::Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            e.exit_code()
        }
    }
}
