use crate::commands;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hasura-track")]
#[command(author = "scriptonist")]
#[command(version = "0.1")]
#[command(about = "CLI plugin which allows to database entities in hasura")]
pub struct Cli {
    #[arg(long, env = "HASURA_GRAPHQL_ENDPOINT")]
    pub endpoint: String,
    #[arg(long, env = "HASURA_GRAPHQL_ADMIN_SECRET")]
    pub admin_secret: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Tables(commands::tables::Cmd),
}

impl Cli {}

pub async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Tables(cmd) => cmd.run(&cli).await?,
    };

    Ok(())
}
