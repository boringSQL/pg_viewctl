WITH RECURSIVE direct_deps AS (
    SELECT DISTINCT dc.oid AS dep_oid, dn.nspname::text AS dep_schema, dc.relname::text AS dep_view
    FROM pg_depend dep
    JOIN pg_rewrite rw ON dep.classid = 'pg_rewrite'::regclass AND dep.objid = rw.oid
    JOIN pg_class dc ON rw.ev_class = dc.oid
    JOIN pg_namespace dn ON dc.relnamespace = dn.oid
    JOIN pg_class sc ON dep.refobjid = sc.oid
    JOIN pg_namespace sn ON sc.relnamespace = sn.oid
    JOIN pg_attribute sa ON dep.refobjid = sa.attrelid AND dep.refobjsubid = sa.attnum
    WHERE
      sn.nspname = $1 AND sc.relname = $2 AND sa.attname = $3
      AND NOT sa.attisdropped AND sa.attnum > 0
      AND dc.relkind IN ('v', 'm')
      AND dc.oid <> sc.oid
),
dep_graph (level, dep_schema, dep_name, dep_oid) AS (
    SELECT 1, dep_schema, dep_view, dep_oid FROM direct_deps
    UNION
    SELECT DISTINCT
        dg.level + 1, dep_ns.nspname::text, dep_cl.relname::text, dep_cl.oid
    FROM dep_graph dg
    JOIN pg_depend d ON d.refobjid = dg.dep_oid
    JOIN pg_rewrite rw ON d.classid = 'pg_rewrite'::regclass AND d.objid = rw.oid
    JOIN pg_class dep_cl ON rw.ev_class = dep_cl.oid
    JOIN pg_namespace dep_ns ON dep_cl.relnamespace = dep_ns.oid
    WHERE
      dep_cl.relkind IN ('v', 'm') AND dep_cl.oid <> dg.dep_oid
)
SELECT max(level)::int AS level, dep_schema, dep_name AS dep_view,
       c.relkind::text AS view_kind,
       pgvc_get_view_definition(dep_schema, dep_name) AS view_def
FROM dep_graph
JOIN pg_class c ON c.relname = dep_name
JOIN pg_namespace n ON c.relnamespace = n.oid AND n.nspname = dep_schema
GROUP BY dep_schema, dep_name, c.relkind
ORDER BY max(level), dep_schema, dep_name
