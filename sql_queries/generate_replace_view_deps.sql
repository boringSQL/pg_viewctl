SELECT d.level, d.dep_schema, d.dep_view,
       c.relkind::text AS view_kind,
       pgvc_get_view_definition(d.dep_schema, d.dep_view) AS view_def
FROM pgvc_dependency_order($1, $2) d
JOIN pg_class c ON c.relname = d.dep_view
JOIN pg_namespace n ON c.relnamespace = n.oid AND n.nspname = d.dep_schema
WHERE d.level > 0
ORDER BY d.level, d.dep_schema, d.dep_view
