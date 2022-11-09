use crate::commands;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hasura-track")]
#[command(author = "scriptonist")]
#[command(version = "0.1")]
#[command(about = "Hasura CLI plugin that help you track stuff")]
pub struct Cli {
    #[arg(long)]
    pub endpoint: String,
    #[arg(long)]
    pub admin_secret: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Table(commands::tables::Cmd),
}

impl Cli {}

pub async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Table(cmd) => cmd.run(&cli).await?,
    };

    Ok(())
}
