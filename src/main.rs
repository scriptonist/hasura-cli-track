use anyhow::Result;

mod cli;
mod commands;
mod hasura;

#[tokio::main]
async fn main() -> Result<()> {
    cli::main().await
}
