use std::collections::HashMap;

use super::sql;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct FKInfo {
    column_mapping: HashMap<String, String>,
    constraint_name: String,
    on_delete: String,
    on_update: String,
    ref_table: String,
    ref_table_table_schema: String,
    table_name: String,
    table_schema: String,
}
#[derive(Serialize, Deserialize)]
struct PGRunSQLResponse {
    result: Vec<serde_json::Value>,
    result_type: String,
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
        #[derive(Serialize, Deserialize)]
        struct Args {
            sql: String,
            source: String,
        }
        let r: Request<Args> = Request {
            args: Some(Args {
                sql: sql::get_tables_names(),
                source: database_name.to_string(),
            }),
            type_: "run_sql".to_string(),
        };

        let response: PGRunSQLResponse = self.send(self.endpoint.join("v2/query")?, r).await?;
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
        let _: serde_json::Value = self.send(self.endpoint.join("v1/metadata")?, req).await?;

        Ok(true)
    }

    pub async fn get_foreign_key_relations(
        &self,
        database_name: &str,
        schemas: Vec<String>,
        tables: Vec<TableInfo>,
    ) -> Result<Vec<FKInfo>> {
        #[derive(Serialize, Deserialize)]
        struct Args {
            sql: String,
            source: String,
        }
        let r: Request<Args> = Request {
            args: Some(Args {
                sql: sql::get_fk_relations(schemas, tables),
                source: database_name.to_string(),
            }),
            type_: "run_sql".to_string(),
        };

        let response: PGRunSQLResponse = self.send(self.endpoint.join("v2/query")?, r).await?;
        let fk_info_strs: Vec<String> = serde_json::from_value(response.result[1].clone())?;

        let fk_infos_all: serde_json::Value = serde_json::from_str(&fk_info_strs[0])?;
        let fk_infos: Vec<FKInfo> = serde_json::from_str(&fk_info_strs[0])?;
        println!("{}", serde_json::to_string_pretty(&fk_infos_all)?);
        Ok(fk_infos)
    }

    pub async fn track_relationships(
        &self,
        database_name: &str,
        fk_infos: Vec<FKInfo>,
    ) -> Result<()> {
        // for a table we have to create an object relationship and array relationship
        #[derive(Serialize, Deserialize, Debug)]
        struct TableInfo {
            name: String,
            schema: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct UsingObjectRelationShip {
            foreign_key_constraint_on: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct ForeignKeyConstraintOnArrayRelationship {
            table: TableInfo,
            column: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct UsingArrayRelationShip {
            foreign_key_constraint_on: ForeignKeyConstraintOnArrayRelationship,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct PGCreateObjectRelationShipArgs {
            name: String,
            table: TableInfo,
            using: UsingObjectRelationShip,
            source: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct PGCreateArrayRelationShipArgs {
            name: String,
            table: TableInfo,
            using: UsingArrayRelationShip,
            source: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        enum CreateRelationshipRequest {
            ArrayRelationShip(Request<PGCreateArrayRelationShipArgs>),
            ObjectRelationShip(Request<PGCreateObjectRelationShipArgs>),
        }
        for fk in fk_infos {
            let mut requests: Vec<CreateRelationshipRequest> = vec![];
            let create_obj_releation_ship_args = PGCreateObjectRelationShipArgs {
                name: fk.table_name.clone(),
                source: database_name.to_owned(),
                table: TableInfo {
                    name: fk.ref_table.clone(),
                    schema: fk.ref_table_table_schema.clone(),
                },
                using: UsingObjectRelationShip {
                    foreign_key_constraint_on: fk
                        .column_mapping
                        .values()
                        .nth(0)
                        .ok_or(anyhow!("expected to find column mapping"))?
                        .to_owned(),
                },
            };
            let create_array_relationship_args = PGCreateArrayRelationShipArgs {
                name: fk.ref_table.clone(),
                source: database_name.to_string(),
                table: TableInfo {
                    name: fk.table_name.clone(),
                    schema: fk.table_schema,
                },
                using: UsingArrayRelationShip {
                    foreign_key_constraint_on: ForeignKeyConstraintOnArrayRelationship {
                        column: fk
                            .column_mapping
                            .values()
                            .nth(0)
                            .ok_or(anyhow!("expected to find column mapping"))?
                            .to_owned(),
                        table: TableInfo {
                            name: fk.ref_table,
                            schema: fk.ref_table_table_schema,
                        },
                    },
                },
            };
            requests.push(CreateRelationshipRequest::ObjectRelationShip(Request {
                type_: "pg_create_object_relationship".to_string(),
                args: Some(create_obj_releation_ship_args),
            }));
            requests.push(CreateRelationshipRequest::ArrayRelationShip(Request {
                type_: "pg_create_array_relationship".to_string(),
                args: Some(create_array_relationship_args),
            }));

            // make request
            let r: Request<Vec<CreateRelationshipRequest>> = Request {
                type_: "bulk".to_string(),
                args: Some(requests),
            };

            let _: serde_json::Value = self.send(self.endpoint.join("v1/metadata")?, r).await?;
        }
        Ok(())
    }
}
