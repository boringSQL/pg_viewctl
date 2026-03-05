use pgrx::prelude::*;
use pgrx::datum::{Date, DatumWithOid};
use pgrx::spi::SpiClient;

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

impl DepInfo {
    fn qualified_name(&self) -> String {
        format!(
            "{}.{}",
            pg_quote_ident(&self.dep_schema),
            pg_quote_ident(&self.dep_view),
        )
    }

    fn kind_label(&self) -> &'static str {
        if self.view_kind == "m" {
            "MATERIALIZED VIEW"
        } else {
            "VIEW"
        }
    }
}

type StepRow = (i32, Option<String>, Option<String>);

struct MigrationPlan {
    steps: Vec<StepRow>,
    step_num: i32,
}

impl MigrationPlan {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            step_num: 1,
        }
    }

    fn add(&mut self, operation: &str, sql: String) {
        self.steps
            .push((self.step_num, Some(operation.to_string()), Some(sql)));
        self.step_num += 1;
    }

    fn drop_dependents(&mut self, deps: &[DepInfo]) {
        for dep in deps.iter().rev() {
            let kind = dep.kind_label();
            let qualified = dep.qualified_name();
            self.add(
                &format!("DROP {kind}"),
                format!("DROP {kind} IF EXISTS {qualified}"),
            );
        }
    }

    fn create_dependents(
        &mut self,
        deps: &[DepInfo],
        annotate: impl Fn(&DepInfo) -> Option<String>,
    ) {
        for dep in deps {
            let create_sql = match annotate(dep) {
                Some(comment) => format!("{comment}\n{}", dep.view_def),
                None => dep.view_def.clone(),
            };
            self.add(&format!("CREATE {}", dep.kind_label()), create_sql);
        }
    }

    fn restore_grants(&mut self, grants: &[String]) {
        for grant in grants {
            self.add("GRANT", grant.clone());
        }
    }

    fn refresh_matviews(&mut self, qualified_target: Option<&str>, deps: &[DepInfo]) {
        if let Some(target) = qualified_target {
            self.add(
                "REFRESH MATERIALIZED VIEW",
                format!("REFRESH MATERIALIZED VIEW {target}"),
            );
        }
        for dep in deps {
            if dep.view_kind == "m" {
                self.add(
                    "REFRESH MATERIALIZED VIEW",
                    format!("REFRESH MATERIALIZED VIEW {}", dep.qualified_name()),
                );
            }
        }
    }

    fn into_table_iter(
        self,
    ) -> TableIterator<
        'static,
        (
            name!(step, i32),
            name!(operation, Option<String>),
            name!(sql, Option<String>),
        ),
    > {
        TableIterator::new(self.steps)
    }
}

fn fetch_deps(
    client: &SpiClient<'_>,
    sql: &str,
    args: &[DatumWithOid],
) -> Result<Vec<DepInfo>, spi::SpiError> {
    let rows = client.select(sql, None, args)?;
    let mut deps = Vec::new();
    for row in rows {
        deps.push(DepInfo {
            dep_schema: row.get_by_name::<String, _>("dep_schema")?.unwrap_or_default(),
            dep_view: row.get_by_name::<String, _>("dep_view")?.unwrap_or_default(),
            view_kind: row.get_by_name::<String, _>("view_kind")?.unwrap_or_default(),
            view_def: row.get_by_name::<String, _>("view_def")?.unwrap_or_default(),
        });
    }
    Ok(deps)
}

fn fetch_col_refs(
    client: &SpiClient<'_>,
    sql: &str,
    args: &[DatumWithOid],
) -> Result<Vec<(String, String)>, spi::SpiError> {
    let rows = client.select(sql, None, args)?;
    let mut refs = Vec::new();
    for row in rows {
        refs.push((
            row.get_by_name::<String, _>("dep_schema")?.unwrap_or_default(),
            row.get_by_name::<String, _>("dep_view")?.unwrap_or_default(),
        ));
    }
    Ok(refs)
}

fn fetch_grants(
    client: &SpiClient<'_>,
    sql: &str,
    args: &[DatumWithOid],
) -> Result<Vec<String>, spi::SpiError> {
    let rows = client.select(sql, None, args)?;
    let mut grants = Vec::new();
    for row in rows {
        if let Some(sql) = row.get_by_name::<String, _>("grant_sql")? {
            grants.push(sql);
        }
    }
    Ok(grants)
}

fn dep_references_column(
    col_refs: &[(String, String)],
    dep: &DepInfo,
) -> bool {
    col_refs
        .iter()
        .any(|(s, v)| s == &dep.dep_schema && v == &dep.dep_view)
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

    Spi::connect(|client| {
        let target_row = client.select(target_kind_sql, Some(1), &args)?;
        if target_row.len() == 0 {
            pgrx::error!("view {schema_name}.{view_name} does not exist");
        }
        let target_kind: String = target_row
            .first()
            .get_by_name::<String, _>("view_kind")?
            .unwrap_or_default();

        let deps = fetch_deps(&client, deps_sql, &args)?;
        let grants = fetch_grants(&client, grants_sql, &args)?;

        let qualified_target = format!(
            "{}.{}",
            pg_quote_ident(schema_name),
            pg_quote_ident(view_name)
        );

        let mut plan = MigrationPlan::new();
        plan.drop_dependents(&deps);

        // DROP+CREATE when matview or when dependents exist (column list may change)
        // CREATE OR REPLACE only when standalone regular view (safe, preserves grants)
        let needs_drop = target_kind == "m" || !deps.is_empty();
        if needs_drop {
            let kind = if target_kind == "m" {
                "MATERIALIZED VIEW"
            } else {
                "VIEW"
            };
            plan.add(
                &format!("DROP {kind}"),
                format!("DROP {kind} IF EXISTS {qualified_target}"),
            );
            plan.add(
                &format!("CREATE {kind}"),
                format!("CREATE {kind} {qualified_target} AS\n{trimmed}"),
            );
        } else {
            plan.add(
                "CREATE OR REPLACE VIEW",
                format!("CREATE OR REPLACE VIEW {qualified_target} AS\n{trimmed}"),
            );
        }

        plan.create_dependents(&deps, |_| None);
        plan.restore_grants(&grants);

        let target_refresh = if target_kind == "m" {
            Some(qualified_target.as_str())
        } else {
            None
        };
        plan.refresh_matviews(target_refresh, &deps);

        Ok::<_, spi::SpiError>(plan.into_table_iter())
    })
    .unwrap()
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

    Spi::connect(|client| {
        let validate_rows = client.select(validate_sql, Some(1), &col_args)?;
        if validate_rows.len() == 0 {
            pgrx::error!(
                "table {schema_name}.{table_name} column {column_name} does not exist"
            );
        }

        let deps = fetch_deps(&client, deps_sql, &col_args)?;
        let col_refs = fetch_col_refs(&client, col_refs_sql, &col_args)?;
        let grants = fetch_grants(&client, grants_sql, &col_args)?;

        let qualified_table = format!(
            "{}.{}",
            pg_quote_ident(schema_name),
            pg_quote_ident(table_name)
        );

        let mut plan = MigrationPlan::new();
        plan.drop_dependents(&deps);
        plan.add(
            "ALTER TABLE",
            format!(
                "ALTER TABLE {qualified_table} DROP COLUMN {}",
                pg_quote_ident(column_name)
            ),
        );
        plan.create_dependents(&deps, |dep| {
            if dep_references_column(&col_refs, dep) {
                Some(format!("-- TODO: remove reference to '{column_name}'"))
            } else {
                None
            }
        });
        plan.restore_grants(&grants);
        plan.refresh_matviews(None, &deps);

        Ok::<_, spi::SpiError>(plan.into_table_iter())
    })
    .unwrap()
}

