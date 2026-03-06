use postgres::{Client, NoTls};

pub fn connect(dsn: Option<&str>) -> Client {
    let mut client = match dsn {
        Some(url) => {
            let result = Client::connect(url, NoTls);
            match result {
                Ok(c) => c,
                Err(e) => panic!("failed to connect to PostgreSQL: {e}"),
            }
        }
        None => {
            let config = "".parse::<postgres::Config>().unwrap();
            match config.connect(NoTls) {
                Ok(c) => c,
                Err(e) => panic!("failed to connect to PostgreSQL: {e}"),
            }
        }
    };

    let row = client.query_opt(
        "SELECT n.nspname FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace WHERE e.extname = 'pg_viewctl'",
        &[],
    ).unwrap();

    match row {
        Some(row) => {
            let schema: String = row.get(0);
            client.execute(&format!("SET search_path TO {}, public", schema), &[]).unwrap();
        }
        None => panic!("pg_viewctl extension is not installed in the database"),
    }

    client
}
