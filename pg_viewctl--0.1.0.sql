\echo Use "CREATE EXTENSION pg_viewctl" to load this file. \quit

-- pg_viewctl extension
-- Version 0.1.0

-- dprecation tracking table
CREATE TABLE IF NOT EXISTS pgvc_deprecated_columns (
    schema_name name NOT NULL,
    view_name name NOT NULL,
    column_name name NOT NULL,
    deprecation_message text,
    removal_date date,
    deprecated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (schema_name, view_name, column_name)
);

CREATE FUNCTION pgvc_map_deptype(deptype "char") RETURNS text AS $$
SELECT CASE deptype
    WHEN 'n' THEN 'NORMAL'
    WHEN 'a' THEN 'AUTO'
    WHEN 'i' THEN 'INTERNAL'
    WHEN 'e' THEN 'EXTENSION'
    WHEN 'p' THEN 'PIN'
    ELSE 'UNKNOWN'
END;
$$ LANGUAGE sql IMMUTABLE STRICT;

CREATE FUNCTION pgvc_map_impact(deptype "char") RETURNS text AS $$
SELECT CASE deptype
    WHEN 'n' THEN 'BREAKING'
    WHEN 'a' THEN 'WARNING'
    ELSE 'INFO'
END;
$$ LANGUAGE sql IMMUTABLE STRICT;

-- helper view: column-level dependencies across views
CREATE OR REPLACE VIEW pgvc_column_dependencies AS
SELECT
    sn.nspname as source_schema,
    sc.relname as source_object,
    sa.attname as source_column,
    dn.nspname as dependent_schema,
    dc.relname as dependent_view,
    COALESCE(da.attname, sa.attname) as dependent_column,
    pgvc_map_deptype(d.deptype) as dependency_type,
    pgvc_map_impact(d.deptype) as impact_severity
FROM pg_depend d
    JOIN pg_rewrite rw ON d.classid = 'pg_rewrite'::regclass
        AND d.objid = rw.oid
    JOIN pg_class dc ON rw.ev_class = dc.oid
    JOIN pg_namespace dn ON dc.relnamespace = dn.oid
    JOIN pg_class sc ON d.refobjid = sc.oid
    JOIN pg_namespace sn ON sc.relnamespace = sn.oid
    JOIN pg_attribute sa ON d.refobjid = sa.attrelid
        AND d.refobjsubid = sa.attnum
        AND sa.attnum > 0
    LEFT JOIN pg_attribute da ON da.attrelid = dc.oid
        AND da.attname = sa.attname
        AND NOT da.attisdropped
        AND da.attnum > 0
WHERE dc.relkind IN ('v', 'm')
    AND sc.relkind IN ('r', 'v', 'm')
    AND d.refobjsubid > 0
    AND NOT sa.attisdropped
ORDER BY source_schema, source_object, source_column, dependent_schema, dependent_view;

-- helper view: deprecated columns with their dependents
CREATE OR REPLACE VIEW pgvc_deprecated_with_dependents AS
SELECT
    dc.schema_name,
    dc.view_name,
    dc.column_name,
    dc.deprecation_message,
    dc.removal_date,
    dc.deprecated_at,
    COUNT(DISTINCT cd.dependent_schema || '.' || cd.dependent_view) as dependent_count,
    array_agg(DISTINCT cd.dependent_schema || '.' || cd.dependent_view
        ORDER BY cd.dependent_schema || '.' || cd.dependent_view)
        FILTER (WHERE cd.dependent_view IS NOT NULL) as dependents
FROM pgvc_deprecated_columns dc
    LEFT JOIN pgvc_column_dependencies cd
        ON dc.schema_name = cd.source_schema
        AND dc.view_name = cd.source_object
        AND dc.column_name = cd.source_column
GROUP BY
    dc.schema_name,
    dc.view_name,
    dc.column_name,
    dc.deprecation_message,
    dc.removal_date,
    dc.deprecated_at
ORDER BY dc.schema_name, dc.view_name, dc.column_name;

CREATE FUNCTION pgvc_analyze_drop_report(
    p_schema_name text,
    p_view_name   text,
    p_column_name text
) RETURNS text AS $$
SELECT coalesce(
    string_agg(
        d.impact_severity || E'\t' || d.usage_location || '.' || d.dependent_column,
        E'\n' ORDER BY d.impact_severity = 'BREAKING' DESC,
                       d.dependent_view, d.dependent_column
    ),
    'no dependencies'
)
FROM analyze_drop_column(p_schema_name, p_view_name, p_column_name) d;
$$ LANGUAGE sql STABLE STRICT;

CREATE FUNCTION pgvc_dependency_order (p_schema text, p_object text)
    RETURNS TABLE (
        level int,
        dep_schema text,
        dep_view text
    )
    AS $$
    WITH RECURSIVE dep_graph (
        level,
        dep_schema,
        dep_name,
        dep_oid
) AS (
        SELECT
            0,
            n.nspname,
            c.relname,
            c.oid
        FROM
            pg_class c
            JOIN pg_namespace n ON c.relnamespace = n.oid
        WHERE
            n.nspname = p_schema
            AND c.relname = p_object
        UNION
        SELECT DISTINCT
            dg.level + 1,
            dep_ns.nspname,
            dep_cl.relname,
            dep_cl.oid
        FROM
            dep_graph dg
            JOIN pg_depend d ON d.refobjid = dg.dep_oid
            JOIN pg_rewrite rw ON d.classid = 'pg_rewrite'::regclass
                AND d.objid = rw.oid
            JOIN pg_class dep_cl ON rw.ev_class = dep_cl.oid
            JOIN pg_namespace dep_ns ON dep_cl.relnamespace = dep_ns.oid
        WHERE
            dep_cl.relkind IN ('v', 'm')
            AND dep_cl.oid <> dg.dep_oid
)
    SELECT
        max(level)::int,
        dep_schema,
        dep_name
    FROM
        dep_graph
    GROUP BY
        dep_schema,
        dep_name
    ORDER BY
        max(level),
        dep_schema,
        dep_name;
$$
LANGUAGE sql
STABLE STRICT;

CREATE FUNCTION pgvc_get_view_definition(p_schema text, p_view text)
RETURNS text AS $$
SELECT
    'CREATE ' || CASE c.relkind
        WHEN 'm' THEN 'MATERIALIZED '
        ELSE ''
        END || 'VIEW ' || quote_ident(n.nspname) || '.' || quote_ident(c.relname) || ' AS' || E'\n' || pg_get_viewdef(c.oid, true)
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
WHERE
  n.nspname = p_schema
  AND c.relname = p_view
  AND c.relkind IN ('v', 'm');
$$ LANGUAGE sql STABLE STRICT;

CREATE FUNCTION pgvc_get_view_grants(p_schema text, p_view text)
RETURNS TABLE (
    grantee text,
    privilege_type text,
    is_grantable boolean
) AS $$
SELECT
    COALESCE(r.rolname, 'PUBLIC'),
    a.privilege_type,
    a.is_grantable
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
CROSS JOIN LATERAL aclexplode(c.relacl) a
LEFT JOIN pg_roles r ON a.grantee = r.oid
WHERE n.nspname = p_schema
  AND c.relname = p_view
  AND c.relkind IN ('v', 'm')
  AND a.grantee <> c.relowner
$$ LANGUAGE sql STABLE STRICT;

