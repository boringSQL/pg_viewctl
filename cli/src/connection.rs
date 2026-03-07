use postgres::{Client, NoTls};

pub fn connect(dsn: Option<&str>) -> Result<Client, String> {
    let mut client = match dsn {
        Some(url) => Client::connect(url, NoTls)
            .map_err(|e| format!("failed to connect to PostgreSQL: {e}"))?,
        None => {
            let config = "".parse::<postgres::Config>().unwrap();
            config
                .connect(NoTls)
                .map_err(|e| format!("failed to connect to PostgreSQL: {e}"))?
        }
    };

    let row = client
        .query_opt(
            "SELECT n.nspname FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace WHERE e.extname = 'pg_viewctl'",
            &[],
        )
        .map_err(|e| format!("failed to check for pg_viewctl extension: {e}"))?;

    match row {
        Some(row) => {
            let schema: String = row.get(0);
            client.execute(&format!("SET search_path TO {}, public", schema), &[]).unwrap();
        }
        None => return Err("pg_viewctl extension is not installed in the database".to_string()),
    }

    Ok(client)
}
