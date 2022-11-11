use crate::hasura;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct Cmd {
    #[arg(long)]
    pub database_name: String,
}
impl Cmd {
    pub async fn run(&self, cli: &crate::cli::Cli) -> Result<()> {
        let client = hasura::Client::new(cli.endpoint.clone(), cli.admin_secret.clone())?;
        let tables = client.get_table_names(&self.database_name).await?;
        let schemas: Vec<String> = tables
            .iter()
            .map(|table| table.table_schema.clone())
            .collect();
        let fk_infos = client
            .get_foreign_key_relations(&self.database_name, schemas, tables)
            .await?;
        let _ = client
            .track_relationships(&self.database_name, fk_infos)
            .await?;
        Ok(())
    }
}
