use anyhow::{Context, Result};
use postgres::Client;

use super::MigrationStep;
use crate::Operation;

pub fn run(client: &mut Client, operation: &Operation) -> Result<Vec<MigrationStep>> {
    let rows = match operation {
        Operation::DropColumn { target } => {
            let t = crate::parse::parse_three_part(target)?;
            client.query(
                "SELECT step, operation, sql, target FROM generate_drop_column($1, $2, $3)",
                &[&t.schema, &t.object, &t.column],
            )
        }
        Operation::ReplaceView { target, definition } => {
            let t = crate::parse::parse_two_part(target)?;
            let def = read_definition(definition)?;
            client.query(
                "SELECT step, operation, sql, target FROM generate_replace_view($1, $2, $3)",
                &[&t.schema, &t.object, &def],
            )
        }
        Operation::AlterType { target, new_type } => {
            let t = crate::parse::parse_three_part(target)?;
            client.query(
                "SELECT step, operation, sql, target FROM generate_alter_type($1, $2, $3, $4)",
                &[&t.schema, &t.object, &t.column, new_type],
            )
        }
        Operation::RenameViewColumn { target, new_name } => {
            let t = crate::parse::parse_three_part(target)?;
            client.query(
                "SELECT step, operation, sql, target FROM generate_rename_view_column($1, $2, $3, $4)",
                &[&t.schema, &t.object, &t.column, new_name],
            )
        }
    }
    .context("query failed")?;

    Ok(rows
        .iter()
        .map(|row| MigrationStep {
            step: row.get(0),
            operation: row.get(1),
            sql: row.get(2),
            target: row.get(3),
        })
        .collect())
}

fn read_definition(source: &str) -> Result<String> {
    if source == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(source)
            .with_context(|| format!("failed to read '{source}'"))
    }
}
