use crate::hasura;
use anyhow::Result;
use clap::Args;
use console::style;
use console::Emoji;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};

static HEAVY_CHECK: Emoji<'_, '_> = Emoji("✅ ", ":-)");
static CROSS: Emoji<'_, '_> = Emoji("❌  ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

#[derive(Args)]
pub struct Cmd {
    #[arg(long)]
    pub database_name: String,
}
impl Cmd {
    pub async fn run(&self, cli: &crate::cli::Cli) -> Result<()> {
        let client = hasura::Client::new(cli.endpoint.clone(), cli.admin_secret.clone())?;
        let tables = client.get_table_names(&self.database_name).await?;
        let pb = ProgressBar::new(tables.len() as u64);
        let started = Instant::now();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg}")
                .unwrap()
                // For more spinners check out the cli-spinners project:
                // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
                .tick_strings(&[
                    "▹▹▹▹▹",
                    "▸▹▹▹▹",
                    "▹▸▹▹▹",
                    "▹▹▸▹▹",
                    "▹▹▹▸▹",
                    "▹▹▹▹▸",
                    "▪▪▪▪▪",
                ]),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        let mut errors: Vec<anyhow::Error> = vec![];
        for (idx, table) in tables.iter().enumerate() {
            let prepend_emoji = |emoji| {
                format!(
                    "{}/{} {} {}.{}",
                    style(idx + 1),
                    style(tables.len()),
                    emoji,
                    style(&table.table_schema).bold().dim(),
                    style(&table.table_name).bold(),
                )
            };
            match client
                .track_pg_table(
                    self.database_name.clone(),
                    table.table_name.clone(),
                    table.table_schema.clone(),
                )
                .await
            {
                Ok(_) => {
                    pb.println(prepend_emoji(HEAVY_CHECK));
                }
                Err(e) => {
                    errors.push(anyhow::anyhow!(
                        "failed tracking {}.{} {}",
                        &table.table_schema,
                        &table.table_name,
                        e
                    ));
                    pb.println(prepend_emoji(CROSS));
                }
            };
        }
        if errors.is_empty() {
            pb.finish_with_message(format!(
                "{} Done ({})",
                SPARKLE,
                HumanDuration(started.elapsed())
            ));
        } else {
            pb.finish_with_message(format!(
                "{} Finished with errors. ({})",
                CROSS,
                HumanDuration(started.elapsed())
            ));
            for e in errors {
                eprintln!("{}", e)
            }
        }
        Ok(())
    }
}
