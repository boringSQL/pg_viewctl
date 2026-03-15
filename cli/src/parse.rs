pub struct SchemaObject {
    pub schema: String,
    pub object: String,
}

pub struct SchemaObjectColumn {
    pub schema: String,
    pub object: String,
    pub column: String,
}

pub fn parse_two_part(target: &str) -> anyhow::Result<SchemaObject> {
    let parts: Vec<&str> = target.split('.').collect();
    anyhow::ensure!(parts.len() == 2, "expected schema.object, got '{target}'");
    Ok(SchemaObject {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
    })
}

pub fn parse_three_part(target: &str) -> anyhow::Result<SchemaObjectColumn> {
    let parts: Vec<&str> = target.split('.').collect();
    anyhow::ensure!(parts.len() == 3, "expected schema.object.column, got '{target}'");
    Ok(SchemaObjectColumn {
        schema: parts[0].to_string(),
        object: parts[1].to_string(),
        column: parts[2].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_two_part() {
        let r = parse_two_part("public.my_view").unwrap();
        assert_eq!(r.schema, "public");
        assert_eq!(r.object, "my_view");
    }

    #[test]
    fn test_parse_two_part_wrong_count() {
        assert!(parse_two_part("public").is_err());
    }

    #[test]
    fn test_parse_three_part() {
        let r = parse_three_part("public.employees.salary").unwrap();
        assert_eq!(r.schema, "public");
        assert_eq!(r.object, "employees");
        assert_eq!(r.column, "salary");
    }

    #[test]
    fn test_parse_three_part_wrong_count() {
        assert!(parse_three_part("public.employees").is_err());
    }
}
