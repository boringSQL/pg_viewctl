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
}

fn main() {
    let _cli = Cli::parse();
}
