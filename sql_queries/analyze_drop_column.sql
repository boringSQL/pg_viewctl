WITH target_view AS (
    SELECT c.oid
    FROM pg_class c
    JOIN pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname = $1
      AND c.relname = $2
      AND c.relkind IN ('v', 'm')
),
target_column AS (
    SELECT a.attnum, a.attname
    FROM pg_attribute a
    JOIN target_view tv ON a.attrelid = tv.oid
    WHERE a.attname = $3
      AND NOT a.attisdropped
      AND a.attnum > 0
),
column_deps AS (
    SELECT dep_ns.nspname AS dependent_schema,
           dep_cl.relname AS dependent_view,
           COALESCE(dep_at.attname, tc.attname) AS dependent_column,
           d.deptype
    FROM pg_depend d
    JOIN pg_rewrite rw ON d.classid = 'pg_rewrite'::regclass
        AND d.objid = rw.oid
    JOIN target_view tv ON d.refobjid = tv.oid
    JOIN target_column tc ON d.refobjsubid = tc.attnum
    JOIN pg_class dep_cl ON rw.ev_class = dep_cl.oid
    JOIN pg_namespace dep_ns ON dep_cl.relnamespace = dep_ns.oid
    LEFT JOIN pg_attribute dep_at ON dep_at.attrelid = dep_cl.oid
        AND dep_at.attname = tc.attname
        AND NOT dep_at.attisdropped
        AND dep_at.attnum > 0
    WHERE d.deptype IN ('n', 'a')
      AND dep_cl.relkind IN ('v', 'm')
)
SELECT dependent_view, dependent_column,
       pgvc_map_deptype(deptype) AS usage_type,
       pgvc_map_impact(deptype) AS impact_severity,
       dependent_schema || '.' || dependent_view AS usage_location
FROM column_deps
ORDER BY dependent_view, dependent_column
