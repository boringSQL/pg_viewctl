SELECT 1
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
JOIN pg_attribute a ON a.attrelid = c.oid
WHERE n.nspname = $1
  AND c.relname = $2
  AND a.attname = $3
  AND c.relkind IN ('v', 'm')
  AND NOT a.attisdropped
  AND a.attnum > 0
