-- SQL functions that the generate_alter_type queries depend on
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

-- Base table with typed columns
CREATE TABLE public.at_base (
    id int,
    amount numeric(10,2),
    label text,
    active boolean
);

-- View that references amount (will get TODO marker when changing type)
CREATE VIEW public.at_view_with_amount AS
    SELECT id, amount FROM public.at_base WHERE active;

-- View that does NOT reference amount (should be excluded from plan)
CREATE VIEW public.at_view_no_amount AS
    SELECT id, label FROM public.at_base WHERE active;

-- Level 2 dependent on at_view_with_amount
CREATE VIEW public.at_dep_l2 AS
    SELECT id, amount FROM public.at_view_with_amount;

-- Materialized view referencing amount
CREATE MATERIALIZED VIEW public.at_mat_dep AS
    SELECT id, amount FROM public.at_base;

-- Grants on affected views
GRANT SELECT ON public.at_view_with_amount TO PUBLIC;
