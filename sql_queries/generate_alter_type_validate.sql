SELECT format_type(a.atttypid, a.atttypmod)::text AS current_type
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
JOIN pg_attribute a ON a.attrelid = c.oid
WHERE
  n.nspname = $1
  AND c.relname = $2
  AND a.attname = $3
  AND c.relkind = 'r'
  AND NOT a.attisdropped
  AND a.attnum > 0
