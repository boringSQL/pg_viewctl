use pgrx::prelude::*;
use pgrx::datum::{Date, DatumWithOid};

fn text_arg(val: &str) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::TEXTOID.into()) }
}

fn optional_text_arg(val: Option<&str>) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::TEXTOID.into()) }
}

fn optional_date_arg(val: Option<Date>) -> DatumWithOid<'static> {
    unsafe { DatumWithOid::new(val.into_datum(), PgBuiltInOids::DATEOID.into()) }
}

::pgrx::pg_module_magic!(name, version);

#[pg_extern]
fn check_column_deprecated(
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
fn deprecate_column(
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
fn get_column_dependencies(
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
fn undeprecate_column(
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
fn analyze_drop_column(
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
fn get_deprecated_columns(
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

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    fn create_deprecated_columns_table() {
        Spi::run(include_str!("../sql_queries/tests/create_deprecated_columns.sql")).unwrap();
    }

    fn create_test_view() {
        Spi::run(include_str!("../sql_queries/tests/create_test_view.sql")).unwrap();
    }

    #[pg_test]
    fn test_check_not_deprecated() {
        create_deprecated_columns_table();

        let result = crate::check_column_deprecated("public", "test_view", "col1");
        assert!(result.is_none());
    }

    #[pg_test]
    fn test_check_deprecated_with_message() {
        create_deprecated_columns_table();

        Spi::run(include_str!("../sql_queries/tests/insert_test_deprecated_column.sql")).unwrap();

        let result = crate::check_column_deprecated("public", "my_view", "old_col");
        assert!(result.is_some());

        let msg = result.unwrap();
        assert!(msg.contains("is deprecated"));
        assert!(msg.contains("Use new_col instead"));
        assert!(msg.contains("2026-06-01"));
    }

    #[pg_test]
    fn test_deprecate_column() {
        create_deprecated_columns_table();
        create_test_view();

        let result = crate::deprecate_column("public", "test_view", "old_col", Some("Use new_col"), None);
        assert_eq!(result, "column public.test_view.old_col deprecated");
    }

    #[pg_test]
    fn test_undeprecate_column() {
        create_deprecated_columns_table();
        create_test_view();

        crate::deprecate_column("public", "test_view", "old_col", Some("going away"), None);
        let result = crate::undeprecate_column("public", "test_view", "old_col");
        assert_eq!(result, "column public.test_view.old_col undeprecated");
    }

    #[pg_test]
    fn test_undeprecate_not_deprecated() {
        create_deprecated_columns_table();

        let result = crate::undeprecate_column("public", "test_view", "col1");
        assert_eq!(result, "column public.test_view.col1 was not marked as deprecated");
    }

    fn create_dependency_fixtures() {
        Spi::run(include_str!("../sql_queries/tests/create_dependency_fixtures.sql")).unwrap();
    }

    #[pg_test]
    fn test_get_column_deps_found() {
        create_dependency_fixtures();

        let results: Vec<_> = crate::get_column_dependencies("public", "test_base")
            .collect();

        let dep_views: Vec<_> = results.iter().filter_map(|r| r.1.as_deref()).collect();
        assert!(
            dep_views.contains(&"test_dep_view"),
            "expected test_dep_view in {:?}, got {} results", dep_views, results.len()
        );

        let source_cols: Vec<_> = results.iter().filter_map(|r| r.3.as_deref()).collect();
        assert!(source_cols.contains(&"id"));
        assert!(source_cols.contains(&"name"));
    }

    #[pg_test]
    fn test_get_column_deps_leaf_view() {
        create_dependency_fixtures();

        let count = Spi::connect(|client| {
            client
                .select(
                    "SELECT count(*) AS cnt FROM get_column_dependencies($1, $2)",
                    None,
                    &[crate::text_arg("public"), crate::text_arg("test_dep_view")],
                )
                .map(|tuptable| {
                    tuptable.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0)
                })
        })
        .unwrap();

        assert_eq!(count, 0);
    }

    #[pg_test]
    fn test_get_column_deps_nonexistent() {
        create_dependency_fixtures();

        let count = Spi::connect(|client| {
            client
                .select(
                    "SELECT count(*) AS cnt FROM get_column_dependencies($1, $2)",
                    None,
                    &[crate::text_arg("public"), crate::text_arg("no_such_thing")],
                )
                .map(|tuptable| {
                    tuptable.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0)
                })
        })
        .unwrap();

        assert_eq!(count, 0);
    }

    #[pg_test]
    #[should_panic(expected = "does not exist")]
    fn test_deprecate_nonexistent_column() {
        create_deprecated_columns_table();
        create_test_view();

        crate::deprecate_column("public", "test_view", "no_such_col", None, None);
    }

    #[pg_test]
    fn test_deprecate_upsert() {
        create_deprecated_columns_table();
        create_test_view();

        crate::deprecate_column("public", "test_view", "old_col", Some("first msg"), None);
        let result = crate::deprecate_column("public", "test_view", "old_col", Some("updated msg"), None);
        assert_eq!(result, "column public.test_view.old_col deprecated");
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
