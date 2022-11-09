use anyhow::Result;
use clap::Parser;

mod cli;
mod hasura;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.run().await?;

    Ok(())
}
