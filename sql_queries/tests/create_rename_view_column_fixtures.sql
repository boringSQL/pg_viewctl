-- SQL functions that the generate_rename_view_column queries depend on
CREATE OR REPLACE FUNCTION pgvc_dependency_order (p_schema text, p_object text)
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

CREATE OR REPLACE FUNCTION pgvc_get_view_definition(p_schema text, p_view text)
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

CREATE OR REPLACE FUNCTION pgvc_get_view_grants(p_schema text, p_view text)
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

-- Base table
CREATE TABLE public.rvc_base (
    id int,
    name text,
    email text,
    active boolean
);

-- Target view whose column gets renamed
CREATE VIEW public.rvc_target AS
    SELECT id, name, email FROM public.rvc_base WHERE active;

-- Dependent that references 'name' (the column we'll rename)
CREATE VIEW public.rvc_dep_l1 AS
    SELECT id, name FROM public.rvc_target;

-- Level 2 dependent
CREATE VIEW public.rvc_dep_l2 AS
    SELECT id, name FROM public.rvc_dep_l1;

-- Materialized view dependent referencing 'name'
CREATE MATERIALIZED VIEW public.rvc_mat_dep AS
    SELECT id, name FROM public.rvc_target;

-- Dependent that does NOT reference 'name'
CREATE VIEW public.rvc_dep_no_ref AS
    SELECT id, email FROM public.rvc_target;

-- Grants
GRANT SELECT ON public.rvc_target TO PUBLIC;
GRANT SELECT ON public.rvc_dep_l1 TO PUBLIC;
