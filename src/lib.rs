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

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    fn create_deprecated_columns_table() {
        Spi::run(include_str!("../sql_queries/create_deprecated_columns.sql")).unwrap();
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

        Spi::run("INSERT INTO pgvc_deprecated_columns
            (schema_name, view_name, column_name, deprecation_message, removal_date)
            VALUES ('public', 'my_view', 'old_col', 'Use new_col instead', '2026-06-01')"
        ).unwrap();

        let result = crate::check_column_deprecated("public", "my_view", "old_col");
        assert!(result.is_some());

        let msg = result.unwrap();
        assert!(msg.contains("is deprecated"));
        assert!(msg.contains("Use new_col instead"));
        assert!(msg.contains("2026-06-01"));
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
