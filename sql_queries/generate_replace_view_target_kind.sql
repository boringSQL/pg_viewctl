SELECT c.relkind::text AS view_kind
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind IN ('v', 'm')
