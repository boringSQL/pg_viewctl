use postgres::Client;

use crate::parse::SchemaObject;

pub fn run(client: &mut Client, target: &SchemaObject) {
    let rows = client
        .query(
            "SELECT level, dep_schema, dep_view FROM pgvc_dependency_order($1, $2) WHERE level > 0",
            &[&target.schema, &target.object],
        )
        .unwrap();

    println!("Dependencies of {}.{}:", target.schema, target.object);

    if rows.is_empty() {
        println!("  (none)");
        println!("\n0 objects affected.");
        return;
    }

    for row in &rows {
        let level: i32 = row.get(0);
        let dep_schema: &str = row.get(1);
        let dep_view: &str = row.get(2);
        println!("  Level {level}: {dep_schema}.{dep_view}");
    }

    println!("\n{} objects affected.", rows.len());
}
