use anyhow::{bail, Context, Result};
use postgres::{Client, NoTls};

pub fn connect(dsn: Option<&str>) -> Result<Client> {
    let mut client = match dsn {
        Some(url) => Client::connect(url, NoTls)
            .context("failed to connect to PostgreSQL")?,
        None => {
            let config = "".parse::<postgres::Config>()
                .context("failed to parse default PostgreSQL config")?;
            config
                .connect(NoTls)
                .context("failed to connect to PostgreSQL")?
        }
    };

    let row = client
        .query_opt(
            "SELECT n.nspname FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace WHERE e.extname = 'pg_viewctl'",
            &[],
        )
        .context("failed to check for pg_viewctl extension")?;

    match row {
        Some(row) => {
            let schema: String = row.get(0);
            client.execute(&format!("SET search_path TO {}, public", schema), &[])
                .context("failed to set search_path")?;
        }
        None => bail!("pg_viewctl extension is not installed in the database"),
    }

    Ok(client)
}
