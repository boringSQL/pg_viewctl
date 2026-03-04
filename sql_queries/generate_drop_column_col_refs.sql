SELECT DISTINCT dn.nspname::text AS dep_schema, dc.relname::text AS dep_view
FROM pg_depend dep
JOIN pg_rewrite rw ON dep.classid = 'pg_rewrite'::regclass AND dep.objid = rw.oid
JOIN pg_class dc ON rw.ev_class = dc.oid
JOIN pg_namespace dn ON dc.relnamespace = dn.oid
JOIN pg_class sc ON dep.refobjid = sc.oid
JOIN pg_namespace sn ON sc.relnamespace = sn.oid
JOIN pg_attribute sa ON dep.refobjid = sa.attrelid AND dep.refobjsubid = sa.attnum
WHERE sn.nspname = $1 AND sc.relname = $2 AND sa.attname = $3
  AND NOT sa.attisdropped AND sa.attnum > 0
  AND dc.relkind IN ('v', 'm')
  AND dc.oid <> sc.oid
