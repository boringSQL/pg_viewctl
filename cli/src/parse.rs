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
        panic!("expected schema.object, got '{target}'");
    }
    SchemaObject {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
    }
}

pub fn parse_three_part(target: &str) -> SchemaObjectColumn {
    let parts: Vec<&str> = target.split('.').collect();
    if parts.len() != 3 {
        panic!("expected schema.object.column, got '{target}'");
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
    #[should_panic]
    fn test_parse_two_part_wrong_count() {
        parse_two_part("public");
    }

    #[test]
    fn test_parse_three_part() {
        let r = parse_three_part("public.employees.salary");
        assert_eq!(r.schema, "public");
        assert_eq!(r.object, "employees");
        assert_eq!(r.column, "salary");
    }

    #[test]
    #[should_panic]
    fn test_parse_three_part_wrong_count() {
        parse_three_part("public.employees");
    }
}
