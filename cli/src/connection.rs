use postgres::{Client, NoTls};

pub fn connect(dsn: Option<&str>) -> Client {
    let mut client = match dsn {
        Some(url) => match Client::connect(url, NoTls) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to connect to PostgreSQL: {e}");
                std::process::exit(1);
            }
        },
        None => {
            let config = "".parse::<postgres::Config>().unwrap();
            match config.connect(NoTls) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: failed to connect to PostgreSQL: {e}");
                    std::process::exit(1);
                }
            }
        }
    };

    let row = client
        .query_opt(
            "SELECT n.nspname FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace WHERE e.extname = 'pg_viewctl'",
            &[],
        )
        .unwrap_or_else(|e| {
            eprintln!("error: failed to check for pg_viewctl extension: {e}");
            std::process::exit(1);
        });

    match row {
        Some(row) => {
            let schema: String = row.get(0);
            client.execute(&format!("SET search_path TO {}, public", schema), &[]).unwrap();
        }
        None => {
            eprintln!("error: pg_viewctl extension is not installed in the database");
            std::process::exit(1);
        }
    }

    client
}
