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
        "SELECT 1 FROM pg_extension WHERE extname = 'pg_viewctl'",
        &[],
    ).unwrap();

    if row.is_none() {
        panic!("pg_viewctl extension is not installed in the database");
    }

    client
}
