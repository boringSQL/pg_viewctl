use pgrx::prelude::*;
use pgrx::datum::{Date, DatumWithOid};

pub(crate) fn text_arg(val: &str) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::TEXTOID.into()) }
}

fn optional_text_arg(val: Option<&str>) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::TEXTOID.into()) }
}

fn optional_date_arg(val: Option<Date>) -> DatumWithOid<'static> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::DATEOID.into()) }
}

fn pg_quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

struct DepInfo {
    dep_schema: String,
    dep_view: String,
    view_kind: String,
    view_def: String,
}

#[pg_extern]
pub fn check_column_deprecated(
    schema_name: &str,
    view_name: &str,
    column_name: &str,
) -> Option<String> {
    let query = include_str!("../sql_queries/check_column_deprecated.sql");

    let args = vec![text_arg(schema_name), text_arg(view_name), text_arg(column_name)];

    Spi::connect(|client| {
        let row = client.select(query, Some(1), &args)?.first();

        let message: Option<String> = row.get_by_name("deprecation_message")?;
        let removal: Option<String> = row.get_by_name("removal_date")?;

        if message.is_none() && removal.is_none() {
            return Ok::<_, spi::SpiError>(None);
        }

        let mut msg = format!(
            "WARNING: column {schema_name}.{view_name}.{column_name} is deprecated"
        );

        if let Some(m) = message {
            msg.push_str(&format!(" — {m}"));
        }

        if let Some(r) = removal {
            msg.push_str(&format!(" (removal: {r})"));
        }

        Ok(Some(msg))
    })
    .unwrap_or(None)
}

#[pg_extern]
pub fn deprecate_column(
    schema_name: &str,
    view_name: &str,
    column_name: &str,
    message: Option<&str>,
    removal_date: Option<Date>,
) -> String {
    let validate_sql = include_str!("../sql_queries/validate_column_exists.sql");
    let upsert_sql = include_str!("../sql_queries/deprecate_column_upsert.sql");

    Spi::connect(|client| {
        let validate_args = vec![
            text_arg(schema_name),
            text_arg(view_name),
            text_arg(column_name),
        ];

        let rows = client.select(validate_sql, Some(1), &validate_args)?;
        if rows.len() == 0 {
            pgrx::error!(
                "column {schema_name}.{view_name}.{column_name} does not exist on any view or materialized view"
            );
        }

        let upsert_args = vec![
            text_arg(schema_name),
            text_arg(view_name),
            text_arg(column_name),
            optional_text_arg(message),
            optional_date_arg(removal_date),
        ];
        client.select(upsert_sql, None, &upsert_args)?;

        Ok::<_, spi::SpiError>(format!(
            "column {schema_name}.{view_name}.{column_name} deprecated"
        ))
    })
    .unwrap()
}

#[pg_extern]
pub fn get_column_dependencies(
    schema_name: &str,
    object_name: &str,
) -> TableIterator<
    'static,
    (
        name!(dependent_schema, Option<String>),
        name!(dependent_view, Option<String>),
        name!(dependent_column, Option<String>),
        name!(source_column, Option<String>),
        name!(dependency_type, Option<String>),
    ),
> {
    let query = include_str!("../sql_queries/get_column_deps.sql");
    let args = vec![text_arg(schema_name), text_arg(object_name)];

    let rows = Spi::connect(|client| {
        let tuptable = client.select(query, None, &args)?;
        let mut rows = Vec::new();
        for row in tuptable {
            rows.push((
                row.get_by_name::<String, _>("dependent_schema")?,
                row.get_by_name::<String, _>("dependent_view")?,
                row.get_by_name::<String, _>("dependent_column")?,
                row.get_by_name::<String, _>("source_column")?,
                row.get_by_name::<String, _>("dependency_type")?,
            ));
        }
        Ok::<_, spi::SpiError>(rows)
    })
    .unwrap_or_default();

    TableIterator::new(rows)
}

#[pg_extern]
pub fn undeprecate_column(
    schema_name: &str,
    view_name: &str,
    column_name: &str,
) -> String {
    let query = include_str!("../sql_queries/undeprecate_column.sql");
    let args = vec![text_arg(schema_name), text_arg(view_name), text_arg(column_name)];

    Spi::connect(|client| {
        let rows = client.select(query, None, &args)?;
        if rows.len() > 0 {
            Ok::<_, spi::SpiError>(format!(
                "column {schema_name}.{view_name}.{column_name} undeprecated"
            ))
        } else {
            Ok(format!(
                "column {schema_name}.{view_name}.{column_name} was not marked as deprecated"
            ))
        }
    })
    .unwrap()
}

#[pg_extern]
pub fn analyze_drop_column(
    schema_name: &str,
    view_name: &str,
    column_name: &str,
) -> TableIterator<
    'static,
    (
        name!(dependent_view, Option<String>),
        name!(dependent_column, Option<String>),
        name!(usage_type, Option<String>),
        name!(impact_severity, Option<String>),
        name!(usage_location, Option<String>),
    ),
> {
    let query = include_str!("../sql_queries/analyze_drop_column.sql");
    let args = vec![
        text_arg(schema_name),
        text_arg(view_name),
        text_arg(column_name),
    ];

    let rows = Spi::connect(|client| {
        let tuptable = client.select(query, None, &args)?;
        let mut rows = Vec::new();
        for row in tuptable {
            rows.push((
                row.get_by_name::<String, _>("dependent_view")?,
                row.get_by_name::<String, _>("dependent_column")?,
                row.get_by_name::<String, _>("usage_type")?,
                row.get_by_name::<String, _>("impact_severity")?,
                row.get_by_name::<String, _>("usage_location")?,
            ));
        }
        Ok::<_, spi::SpiError>(rows)
    })
    .unwrap_or_default();

    TableIterator::new(rows)
}

#[pg_extern]
pub fn get_deprecated_columns(
    schema_filter: Option<&str>,
) -> TableIterator<
    'static,
    (
        name!(schema_name, Option<String>),
        name!(view_name, Option<String>),
        name!(column_name, Option<String>),
        name!(deprecation_message, Option<String>),
        name!(removal_date, Option<String>),
        name!(deprecated_at, Option<String>),
    ),
> {
    let query = include_str!("../sql_queries/get_deprecated_columns.sql");
    let args = vec![optional_text_arg(schema_filter)];

    let rows = Spi::connect(|client| {
        let tuptable = client.select(query, None, &args)?;
        let mut rows = Vec::new();
        for row in tuptable {
            rows.push((
                row.get_by_name::<String, _>("schema_name")?,
                row.get_by_name::<String, _>("view_name")?,
                row.get_by_name::<String, _>("column_name")?,
                row.get_by_name::<String, _>("deprecation_message")?,
                row.get_by_name::<String, _>("removal_date")?,
                row.get_by_name::<String, _>("deprecated_at")?,
            ));
        }
        Ok::<_, spi::SpiError>(rows)
    })
    .unwrap_or_default();

    TableIterator::new(rows)
}

#[pg_extern]
pub fn generate_replace_view(
    schema_name: &str,
    view_name: &str,
    new_definition: &str,
) -> TableIterator<
    'static,
    (
        name!(step, i32),
        name!(operation, Option<String>),
        name!(sql, Option<String>),
    ),
> {
    let target_kind_sql = include_str!("../sql_queries/generate_replace_view_target_kind.sql");
    let deps_sql = include_str!("../sql_queries/generate_replace_view_deps.sql");
    let grants_sql = include_str!("../sql_queries/generate_replace_view_grants.sql");

    let trimmed = new_definition.trim();
    if trimmed.len() >= 6 && trimmed[..6].eq_ignore_ascii_case("create") {
        pgrx::error!(
            "new_definition should be a SELECT body, not a full CREATE statement"
        );
    }

    let args = vec![text_arg(schema_name), text_arg(view_name)];

    let steps = Spi::connect(|client| {
        // 1. validate target exists, get relkind
        let target_row = client.select(target_kind_sql, Some(1), &args)?;
        if target_row.len() == 0 {
            pgrx::error!(
                "view {schema_name}.{view_name} does not exist"
            );
        }
        let target_kind: String = target_row
            .first()
            .get_by_name::<String, _>("view_kind")?
            .unwrap_or_default();

        // 2. fetch dependents
        let dep_rows = client.select(deps_sql, None, &args)?;
        let mut deps: Vec<DepInfo> = Vec::new();
        for row in dep_rows {
            deps.push(DepInfo {
                dep_schema: row.get_by_name::<String, _>("dep_schema")?.unwrap_or_default(),
                dep_view: row.get_by_name::<String, _>("dep_view")?.unwrap_or_default(),
                view_kind: row.get_by_name::<String, _>("view_kind")?.unwrap_or_default(),
                view_def: row.get_by_name::<String, _>("view_def")?.unwrap_or_default(),
            });
        }

        // 3. fetch grants
        let grant_rows = client.select(grants_sql, None, &args)?;
        let mut grants: Vec<String> = Vec::new();
        for row in grant_rows {
            if let Some(sql) = row.get_by_name::<String, _>("grant_sql")? {
                grants.push(sql);
            }
        }

        // 4. compile necessary steps
        let mut steps: Vec<(i32, Option<String>, Option<String>)> = Vec::new();
        let mut step_num: i32 = 1;

        let qualified_target = format!(
            "{}.{}",
            pg_quote_ident(schema_name),
            pg_quote_ident(view_name)
        );

        // DROP dependents leaf-first (descending level)
        for dep in deps.iter().rev() {
            let qualified = format!(
                "{}.{}",
                pg_quote_ident(&dep.dep_schema),
                pg_quote_ident(&dep.dep_view)
            );
            let drop_kind = if dep.view_kind == "m" {
                "MATERIALIZED VIEW"
            } else {
                "VIEW"
            };
            steps.push((
                step_num,
                Some(format!("DROP {drop_kind}")),
                Some(format!("DROP {drop_kind} IF EXISTS {qualified}")),
            ));
            step_num += 1;
        }

        // Replace the target view
        // DROP+CREATE when matview or when dependents exist (column list may change)
        // CREATE OR REPLACE only when standalone regular view (safe, preserves grants)
        let needs_drop = target_kind == "m" || !deps.is_empty();
        if needs_drop {
            let drop_kind = if target_kind == "m" {
                "MATERIALIZED VIEW"
            } else {
                "VIEW"
            };
            let create_kind = drop_kind;
            steps.push((
                step_num,
                Some(format!("DROP {drop_kind}")),
                Some(format!("DROP {drop_kind} IF EXISTS {qualified_target}")),
            ));
            step_num += 1;
            steps.push((
                step_num,
                Some(format!("CREATE {create_kind}")),
                Some(format!(
                    "CREATE {create_kind} {qualified_target} AS\n{trimmed}"
                )),
            ));
            step_num += 1;
        } else {
            steps.push((
                step_num,
                Some("CREATE OR REPLACE VIEW".to_string()),
                Some(format!(
                    "CREATE OR REPLACE VIEW {qualified_target} AS\n{trimmed}"
                )),
            ));
            step_num += 1;
        }

        // CREATE dependents base-first (ascending level)
        for dep in &deps {
            steps.push((
                step_num,
                Some(if dep.view_kind == "m" {
                    "CREATE MATERIALIZED VIEW".to_string()
                } else {
                    "CREATE VIEW".to_string()
                }),
                Some(dep.view_def.clone()),
            ));
            step_num += 1;
        }

        // GRANT for all views that had grants
        for grant in &grants {
            steps.push((
                step_num,
                Some("GRANT".to_string()),
                Some(grant.clone()),
            ));
            step_num += 1;
        }

        // REFRESH MATERIALIZED VIEW for matviews
        // TODO: flag based?
        if target_kind == "m" {
            steps.push((
                step_num,
                Some("REFRESH MATERIALIZED VIEW".to_string()),
                Some(format!("REFRESH MATERIALIZED VIEW {qualified_target}")),
            ));
            step_num += 1;
        }
        for dep in &deps {
            if dep.view_kind == "m" {
                let qualified = format!(
                    "{}.{}",
                    pg_quote_ident(&dep.dep_schema),
                    pg_quote_ident(&dep.dep_view)
                );
                steps.push((
                    step_num,
                    Some("REFRESH MATERIALIZED VIEW".to_string()),
                    Some(format!("REFRESH MATERIALIZED VIEW {qualified}")),
                ));
                step_num += 1;
            }
        }

        Ok::<_, spi::SpiError>(steps)
    })
    .unwrap();

    TableIterator::new(steps)
}

#[pg_extern]
pub fn generate_drop_column(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> TableIterator<
    'static,
    (
        name!(step, i32),
        name!(operation, Option<String>),
        name!(sql, Option<String>),
    ),
> {
    let validate_sql = include_str!("../sql_queries/generate_drop_column_validate.sql");
    let deps_sql = include_str!("../sql_queries/generate_drop_column_deps.sql");
    let col_refs_sql = include_str!("../sql_queries/generate_drop_column_col_refs.sql");
    let grants_sql = include_str!("../sql_queries/generate_drop_column_grants.sql");

    let col_args = vec![
        text_arg(schema_name),
        text_arg(table_name),
        text_arg(column_name),
    ];

    let steps = Spi::connect(|client| {
        // 1. validate table+column exist on a base table
        let validate_rows = client.select(validate_sql, Some(1), &col_args)?;
        if validate_rows.len() == 0 {
            pgrx::error!(
                "table {schema_name}.{table_name} column {column_name} does not exist"
            );
        }

        // 2. fetch column-level dependent views with definitions
        let dep_rows = client.select(deps_sql, None, &col_args)?;
        let mut deps: Vec<DepInfo> = Vec::new();
        for row in dep_rows {
            deps.push(DepInfo {
                dep_schema: row.get_by_name::<String, _>("dep_schema")?.unwrap_or_default(),
                dep_view: row.get_by_name::<String, _>("dep_view")?.unwrap_or_default(),
                view_kind: row.get_by_name::<String, _>("view_kind")?.unwrap_or_default(),
                view_def: row.get_by_name::<String, _>("view_def")?.unwrap_or_default(),
            });
        }

        // 3. identify which views directly reference the dropped column
        let col_ref_rows = client.select(col_refs_sql, None, &col_args)?;
        let mut col_ref_views: Vec<(String, String)> = Vec::new();
        for row in col_ref_rows {
            col_ref_views.push((
                row.get_by_name::<String, _>("dep_schema")?.unwrap_or_default(),
                row.get_by_name::<String, _>("dep_view")?.unwrap_or_default(),
            ));
        }

        // 4. fetch grants for affected views
        let grant_rows = client.select(grants_sql, None, &col_args)?;
        let mut grants: Vec<String> = Vec::new();
        for row in grant_rows {
            if let Some(sql) = row.get_by_name::<String, _>("grant_sql")? {
                grants.push(sql);
            }
        }

        // 5. generate steps
        let mut steps: Vec<(i32, Option<String>, Option<String>)> = Vec::new();
        let mut step_num: i32 = 1;

        let qualified_table = format!(
            "{}.{}",
            pg_quote_ident(schema_name),
            pg_quote_ident(table_name)
        );

        // DROP dependent views leaf-first (descending level)
        for dep in deps.iter().rev() {
            let qualified = format!(
                "{}.{}",
                pg_quote_ident(&dep.dep_schema),
                pg_quote_ident(&dep.dep_view)
            );
            let drop_kind = if dep.view_kind == "m" {
                "MATERIALIZED VIEW"
            } else {
                "VIEW"
            };
            steps.push((
                step_num,
                Some(format!("DROP {drop_kind}")),
                Some(format!("DROP {drop_kind} IF EXISTS {qualified}")),
            ));
            step_num += 1;
        }

        // ALTER TABLE DROP COLUMN
        steps.push((
            step_num,
            Some("ALTER TABLE".to_string()),
            Some(format!(
                "ALTER TABLE {qualified_table} DROP COLUMN {}",
                pg_quote_ident(column_name)
            )),
        ));
        step_num += 1;

        // CREATE dependent views base-first (ascending level)
        for dep in &deps {
            let references_column = col_ref_views
                .iter()
                .any(|(s, v)| s == &dep.dep_schema && v == &dep.dep_view);

            let create_sql = if references_column {
                format!(
                    "-- TODO: remove reference to '{}'\n{}",
                    column_name, dep.view_def
                )
            } else {
                dep.view_def.clone()
            };

            steps.push((
                step_num,
                Some(if dep.view_kind == "m" {
                    "CREATE MATERIALIZED VIEW".to_string()
                } else {
                    "CREATE VIEW".to_string()
                }),
                Some(create_sql),
            ));
            step_num += 1;
        }

        // GRANT privileges
        for grant in &grants {
            steps.push((
                step_num,
                Some("GRANT".to_string()),
                Some(grant.clone()),
            ));
            step_num += 1;
        }

        // REFRESH materialized views
        for dep in &deps {
            if dep.view_kind == "m" {
                let qualified = format!(
                    "{}.{}",
                    pg_quote_ident(&dep.dep_schema),
                    pg_quote_ident(&dep.dep_view)
                );
                steps.push((
                    step_num,
                    Some("REFRESH MATERIALIZED VIEW".to_string()),
                    Some(format!("REFRESH MATERIALIZED VIEW {qualified}")),
                ));
                step_num += 1;
            }
        }

        Ok::<_, spi::SpiError>(steps)
    })
    .unwrap();

    TableIterator::new(steps)
}
