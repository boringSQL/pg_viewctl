pub struct SchemaObject {
    pub schema: String,
    pub object: String,
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
}
