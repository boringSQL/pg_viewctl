pub struct SchemaObject {
    pub schema: String,
    pub object: String,
}

pub struct SchemaObjectColumn {
    pub schema: String,
    pub object: String,
    pub column: String,
}

pub fn parse_two_part(target: &str) -> SchemaObject {
    let parts: Vec<&str> = target.split('.').collect();
    if parts.len() != 2 {
        eprintln!("error: expected schema.object, got '{target}'");
        std::process::exit(1);
    }
    SchemaObject {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
    }
}

pub fn parse_three_part(target: &str) -> SchemaObjectColumn {
    let parts: Vec<&str> = target.split('.').collect();
    if parts.len() != 3 {
        eprintln!("error: expected schema.object.column, got '{target}'");
        std::process::exit(1);
    }
    SchemaObjectColumn {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
        column: parts[2].to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_two_part() {
        let r = parse_two_part("public.my_view");
        assert_eq!(r.schema, "public");
        assert_eq!(r.object, "my_view");
    }

    #[test]
    fn test_parse_three_part() {
        let r = parse_three_part("public.employees.salary");
        assert_eq!(r.schema, "public");
        assert_eq!(r.object, "employees");
        assert_eq!(r.column, "salary");
    }
}
