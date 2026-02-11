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
    COALESCE(da.attname, '<view_level>') as dependent_column,
    pgvc_map_deptype(d.deptype) as dependency_type,
    pgvc_map_impact(d.deptype) as impact_severity
FROM pg_depend d
    JOIN pg_class sc ON d.refobjid = sc.oid
    JOIN pg_namespace sn ON sc.relnamespace = sn.oid
    LEFT JOIN pg_attribute sa ON d.refobjid = sa.attrelid
        AND d.refobjsubid = sa.attnum
        AND sa.attnum > 0
    JOIN pg_class dc ON d.objid = dc.oid
    JOIN pg_namespace dn ON dc.relnamespace = dn.oid
    LEFT JOIN pg_attribute da ON d.objid = da.attrelid
        AND d.objsubid = da.attnum
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

CREATE FUNCTION get_column_dependencies(
    schema_name text,
    object_name text
) RETURNS TABLE (
    dependent_schema text,
    dependent_view   text,
    dependent_column text,
    source_column    text,
    dependency_type  text
) AS 'MODULE_PATHNAME', 'get_column_dependencies'
LANGUAGE C STRICT;

CREATE FUNCTION analyze_drop_column(
    schema_name text,
    view_name   text,
    column_name text
) RETURNS TABLE (
    dependent_view    text,
    dependent_column  text,
    usage_type        text,
    impact_severity   text,
    usage_location    text
) AS 'MODULE_PATHNAME', 'analyze_drop_column'
LANGUAGE C STRICT;

CREATE FUNCTION deprecate_column(
    schema_name  text,
    view_name    text,
    column_name  text,
    message      text DEFAULT NULL,
    removal_date date DEFAULT NULL
) RETURNS text
AS 'MODULE_PATHNAME', 'deprecate_column'
LANGUAGE C;

CREATE FUNCTION undeprecate_column(
    schema_name text,
    view_name   text,
    column_name text
) RETURNS text
AS 'MODULE_PATHNAME', 'undeprecate_column'
LANGUAGE C STRICT;

CREATE FUNCTION get_deprecated_columns(
    schema_filter text DEFAULT NULL
) RETURNS TABLE (
    schema_name         text,
    view_name           text,
    column_name         text,
    deprecation_message text,
    removal_date        text,
    deprecated_at       text
) AS 'MODULE_PATHNAME', 'get_deprecated_columns'
LANGUAGE C;

CREATE FUNCTION check_column_deprecated(
    schema_name text,
    view_name   text,
    column_name text
) RETURNS text
AS 'MODULE_PATHNAME', 'check_column_deprecated'
LANGUAGE C STRICT;

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
