mod commands;
mod connection;
mod output;
mod parse;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "pg_viewctl", version)]
struct Cli {
    #[arg(long, global = true)]
    dsn: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// show dependency order for an object
    Plan {
        #[command(subcommand)]
        operation: Operation,
    },

    /// generate migration SQL
    Generate {
        #[arg(long, default_value = "sql")]
        format: OutputFormat,
        #[command(subcommand)]
        operation: Operation,
    },
}

#[derive(Subcommand, Clone)]
pub enum Operation {
    /// drop a column from a table
    DropColumn { target: String },

    /// replace a view definition
    ReplaceView {
        target: String,
        #[arg(long)]
        definition: String,
    },

    /// alter a column type
    AlterType {
        target: String,
        #[arg(long)]
        new_type: String,
    },

    /// rename a view column
    RenameViewColumn {
        target: String,
        #[arg(long)]
        new_name: String,
    },
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Sql,
    Plain,
    Goose,
    Flyway,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Command::Plan { operation } => {
            let target = plan_target(operation)?;
            let mut client = connection::connect(cli.dsn.as_deref())?;
            commands::plan::run(&mut client, &target)?;
        }
        Command::Generate { format, operation } => {
            let mut client = connection::connect(cli.dsn.as_deref())?;
            let steps = commands::generate::run(&mut client, operation)?;
            let op_name = operation_slug(operation);
            output::emit(&steps, format, &op_name)?;
        }
    }
    Ok(())
}

fn operation_slug(operation: &Operation) -> String {
    match operation {
        Operation::DropColumn { .. } => "drop_column",
        Operation::ReplaceView { .. } => "replace_view",
        Operation::AlterType { .. } => "alter_type",
        Operation::RenameViewColumn { .. } => "rename_view_column",
    }
    .to_string()
}

fn plan_target(operation: &Operation) -> Result<parse::SchemaObject> {
    let target_str = match operation {
        Operation::DropColumn { target } => target,
        Operation::ReplaceView { target, .. } => target,
        Operation::AlterType { target, .. } => target,
        Operation::RenameViewColumn { target, .. } => target,
    };
    let parts: Vec<&str> = target_str.split('.').collect();
    anyhow::ensure!(
        parts.len() >= 2,
        "target must have at least schema.object, got '{target_str}'"
    );
    Ok(parse::SchemaObject {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
    })
}
