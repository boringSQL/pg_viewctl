WITH RECURSIVE direct_deps AS (
    SELECT DISTINCT dc.oid AS dep_oid, dn.nspname::text AS dep_schema, dc.relname::text AS dep_view
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
    WHERE dep_cl.relkind IN ('v', 'm') AND dep_cl.oid <> dg.dep_oid
),
ordered AS (
    SELECT max(level)::int AS level, dep_schema, dep_name AS dep_view
    FROM dep_graph
    GROUP BY dep_schema, dep_name
)
SELECT d.dep_schema, d.dep_view,
       'GRANT ' || g.privilege_type || ' ON ' ||
       quote_ident(d.dep_schema) || '.' || quote_ident(d.dep_view) || ' TO ' ||
       CASE WHEN g.grantee = 'PUBLIC' THEN 'PUBLIC' ELSE quote_ident(g.grantee) END ||
       CASE WHEN g.is_grantable THEN ' WITH GRANT OPTION' ELSE '' END AS grant_sql
FROM ordered d
CROSS JOIN LATERAL pgvc_get_view_grants(d.dep_schema, d.dep_view) g
ORDER BY d.level, d.dep_schema, d.dep_view
