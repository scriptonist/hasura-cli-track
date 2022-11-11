use super::client::TableInfo;

pub fn get_tables_names() -> String {
    /*
     * SQL to retrive:
     * { table_name: string, table_schema: string, columns: string[], column_types: string[] }[].
     *
     * `columns` is an array of column names.
     * `column_types` is an array of column data types.
     */
    r#"
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
    "#.to_string()
}

pub fn get_fk_relations(schemas: Vec<String>, tables: Vec<TableInfo>) -> String {
    format!(
        r#"

SELECT
	COALESCE(json_agg(row_to_json(info)), '[]'::JSON)
FROM (
	SELECT
		q.table_schema::text AS table_schema,
		q.table_name::text AS table_name,
		q.constraint_name::text AS constraint_name,
		min(q.ref_table_table_schema::text) AS ref_table_table_schema,
		min(q.ref_table::text) AS ref_table,
		json_object_agg(ac.attname, afc.attname) AS column_mapping,
		min(q.confupdtype::text) AS on_update,
		min(q.confdeltype::text) AS
		on_delete
	FROM (
		SELECT
			ctn.nspname AS table_schema,
			ct.relname AS table_name,
			r.conrelid AS table_id,
			r.conname AS constraint_name,
			cftn.nspname AS ref_table_table_schema,
			cft.relname AS ref_table,
			r.confrelid AS ref_table_id,
			r.confupdtype,
			r.confdeltype,
			unnest(r.conkey) AS column_id,
			unnest(r.confkey) AS ref_column_id
		FROM
			pg_constraint r
			JOIN pg_class ct ON r.conrelid = ct.oid
			JOIN pg_namespace ctn ON ct.relnamespace = ctn.oid
			JOIN pg_class cft ON r.confrelid = cft.oid
			JOIN pg_namespace cftn ON cft.relnamespace = cftn.oid
    WHERE
      r.contype = 'f'::"char"
      {}
      ) q
		JOIN pg_attribute ac ON q.column_id = ac.attnum
			AND q.table_id = ac.attrelid
		JOIN pg_attribute afc ON q.ref_column_id = afc.attnum
			AND q.ref_table_id = afc.attrelid
		GROUP BY
			q.table_schema,
			q.table_name,
      q.constraint_name) AS info;

        "#,
        generate_where_clause(schemas, tables, "ct.relname", "ctn.nspname", "AND",)
    )
}

fn generate_where_clause(
    schemas: Vec<String>,
    tables: Vec<TableInfo>,
    sql_table_name: &str,
    sql_schema_name: &str,
    clause_prefix: &str,
) -> String {
    let mut where_conditions: Vec<String> = vec![];
    schemas.iter().for_each(|schema_name| {
        where_conditions.push(format!("{} = '{}'", sql_schema_name, schema_name,))
    });
    tables.iter().for_each(|table| {
        where_conditions.push(format!(
            "{} = '{}' and {} = '{}'",
            sql_schema_name, table.table_schema, sql_table_name, table.table_name,
        ))
    });
    if where_conditions.is_empty() {
        return "".to_string();
    }
    let clause: String = format!("{} ({})", clause_prefix, where_conditions.join(" or "));

    clause
}
