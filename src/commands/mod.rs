mod config;
mod fetch;
mod search;

use serde::Serialize;

use crate::cli::{Cli, Command, FetchTarget};
use crate::error::Result;

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Search(args) => search::run(args).await,
        Command::Fetch { target } => match target {
            FetchTarget::Movie(args) => fetch::movie(args).await,
            FetchTarget::Tv(args) => fetch::tv(args).await,
        },
        Command::Config { action } => config::run(action),
    }
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
