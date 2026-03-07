use postgres::Client;

use super::MigrationStep;
use crate::Operation;

pub fn run(client: &mut Client, operation: &Operation) -> Vec<MigrationStep> {
    let rows = match operation {
        Operation::DropColumn { target } => {
            let t = crate::parse::parse_three_part(target);
            client.query(
                "SELECT step, operation, sql FROM generate_drop_column($1, $2, $3)",
                &[&t.schema, &t.object, &t.column],
            )
        }
        Operation::ReplaceView { target, definition } => {
            let t = crate::parse::parse_two_part(target);
            let def = read_definition(definition);
            client.query(
                "SELECT step, operation, sql FROM generate_replace_view($1, $2, $3)",
                &[&t.schema, &t.object, &def],
            )
        }
        Operation::AlterType { target, new_type } => {
            let t = crate::parse::parse_three_part(target);
            client.query(
                "SELECT step, operation, sql FROM generate_alter_type($1, $2, $3, $4)",
                &[&t.schema, &t.object, &t.column, new_type],
            )
        }
        Operation::RenameViewColumn { target, new_name } => {
            let t = crate::parse::parse_three_part(target);
            client.query(
                "SELECT step, operation, sql FROM generate_rename_view_column($1, $2, $3, $4)",
                &[&t.schema, &t.object, &t.column, new_name],
            )
        }
    }
    .unwrap_or_else(|e| {
        eprintln!("error: query failed: {e}");
        std::process::exit(1);
    });

    rows.iter()
        .map(|row| MigrationStep {
            step: row.get(0),
            operation: row.get(1),
            sql: row.get(2),
        })
        .collect()
}

fn read_definition(source: &str) -> String {
    if source == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("error: failed to read from stdin: {e}");
            std::process::exit(1);
        });
        buf
    } else {
        std::fs::read_to_string(source).unwrap_or_else(|e| {
            eprintln!("error: failed to read '{}': {e}", source);
            std::process::exit(1);
        })
    }
}
