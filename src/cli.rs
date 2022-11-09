use super::hasura;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "hasura-track-all")]
#[command(author = "scriptonist")]
#[command(version = "0.1")]
#[command(about = "Hasura CLI plugin to track all tables in a database")]
pub struct Cli {
    #[arg(long)]
    endpoint: String,
    #[arg(long)]
    database_name: String,
    #[arg(long)]
    schema: String,
    #[arg(long)]
    admin_secret: Option<String>,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        let client = hasura::Client::new(self.endpoint.clone(), self.admin_secret.clone())?;
        let tables = client.get_table_names(&self.database_name).await?;
        for table in tables {
            client
                .track_pg_table(
                    self.database_name.clone(),
                    table.table_name.clone(),
                    table.table_schema.clone(),
                )
                .await?;
            println!(
                "tracked {} in {} schema",
                &table.table_name, &table.table_schema
            );
        }
        Ok(())
    }
}
