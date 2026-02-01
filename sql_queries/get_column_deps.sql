SELECT dep_ns.nspname AS dependent_schema,
       dep_cl.relname AS dependent_view,
       dep_at.attname AS dependent_column,
       src_at.attname AS source_column,
       pgvc_map_deptype(d.deptype) AS dependency_type
FROM pg_depend d
JOIN pg_class src_cl ON d.refobjid = src_cl.oid
JOIN pg_namespace src_ns ON src_cl.relnamespace = src_ns.oid
JOIN pg_attribute src_at ON src_at.attrelid = src_cl.oid
    AND src_at.attnum = d.refobjsubid
JOIN pg_class dep_cl ON d.objid = dep_cl.oid
JOIN pg_namespace dep_ns ON dep_cl.relnamespace = dep_ns.oid
JOIN pg_attribute dep_at ON dep_at.attrelid = dep_cl.oid
    AND dep_at.attnum = d.objsubid
WHERE src_ns.nspname = $1
  AND src_cl.relname = $2
  AND d.refobjsubid > 0
  AND d.objsubid > 0
  AND dep_cl.relkind IN ('v', 'm')
  AND NOT src_at.attisdropped
  AND NOT dep_at.attisdropped
ORDER BY dep_ns.nspname, dep_cl.relname, dep_at.attname
