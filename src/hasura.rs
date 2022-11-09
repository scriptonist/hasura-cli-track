use anyhow::Result;
use anyhow::{anyhow, Context};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

pub struct Client {
    endpoint: url::Url,
    admin_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Request<T> {
    args: Option<T>,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TableInfo {
    pub table_name: String,
    pub table_schema: String,
    columns: Vec<String>,
    column_types: Vec<String>,
}
impl Client {
    pub fn new(endpoint: String, admin_secret: Option<String>) -> Result<Self> {
        Ok(Client {
            endpoint: Url::parse(&endpoint)?,
            admin_secret,
        })
    }
    async fn send<A, B>(&self, url: Url, r: Request<A>) -> Result<B>
    where
        A: Serialize,
        B: DeserializeOwned,
    {
        let resp = reqwest::Client::new()
            .post(url)
            .header(
                "x-hasura-admin-secret",
                self.admin_secret.clone().unwrap_or_else(|| "".to_string()),
            )
            .json(&r)
            .send()
            .await?;

        if resp.status() != reqwest::StatusCode::OK {
            return Err(anyhow!(
                "API Request Failed: {}",
                resp.text()
                    .await
                    .context("decoding response body to JSON failed")?
            ));
        }
        let resp = resp.json().await?;

        Ok(resp)
    }

    pub async fn get_table_names(&self, database_name: &str) -> Result<Vec<TableInfo>> {
        /*
         * SQL to retrive:
         * { table_name: string, table_schema: string, columns: string[], column_types: string[] }[].
         *
         * `columns` is an array of column names.
         * `column_types` is an array of column data types.
         */
        let query = r#"
SELECT
	COALESCE(json_agg(row_to_json(info)), '[]'::JSON)
FROM (
	SELECT
		table_name::text,
		table_schema::text,
		ARRAY_AGG("column_name"::text) as columns,
    ARRAY_AGG("data_type"::text) as column_types
	FROM
		information_schema.columns
	WHERE
		table_schema NOT in('information_schema', 'pg_catalog', 'hdb_catalog', '_timescaledb_internal', 'crdb_internal')
		AND table_schema NOT LIKE 'pg_toast%'
		AND table_schema NOT LIKE 'pg_temp_%'
	GROUP BY
		table_name,
		table_schema) AS info;
    "#;
        #[derive(Serialize, Deserialize)]
        struct Args {
            sql: String,
            source: String,
        }
        let r: Request<Args> = Request {
            args: Some(Args {
                sql: query.to_string(),
                source: database_name.to_string(),
            }),
            type_: "run_sql".to_string(),
        };

        #[derive(Serialize, Deserialize)]
        struct Response {
            result: Vec<serde_json::Value>,
            result_type: String,
        }
        let response: Response = self.send(self.endpoint.join("v2/query")?, r).await?;
        if response.result_type != "TuplesOk" {
            return Err(anyhow!(
                "finding tables from db failed: query error {}",
                serde_json::to_string_pretty(&response)?
            ));
        }
        let table_infos_strs: Vec<String> = serde_json::from_value(response.result[1].clone())?;
        let mut table_infos: Vec<TableInfo> = vec![];
        for info in table_infos_strs {
            table_infos = serde_json::from_str(&info)?
        }
        Ok(table_infos)
    }

    pub async fn track_pg_table(
        &self,
        source_name: String,
        table_name: String,
        schema: String,
    ) -> Result<bool> {
        #[derive(Serialize, Deserialize, Debug)]
        struct QualifiedTable {
            name: String,
            schema: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        enum TableName {
            SourceName(String),
            QualifiedTable(QualifiedTable),
        }
        type SourceName = String;

        #[derive(Serialize, Deserialize, Debug)]
        struct RArgs {
            source: SourceName,
            #[serde(rename = "table")]
            table: TableName,
        }
        let req = Request {
            type_: "pg_track_table".to_string(),
            args: Some(RArgs {
                source: source_name as SourceName,
                table: TableName::QualifiedTable(QualifiedTable {
                    name: table_name,
                    schema,
                }),
            }),
        };
        println!("{}", serde_json::to_string_pretty(&req)?);
        self.send(self.endpoint.join("v1/metadata")?, req).await?;

        Ok(true)
    }
}
