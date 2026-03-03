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

CREATE TABLE public.test_base (id int, name text, value numeric);
CREATE VIEW public.test_dep_view AS SELECT id, name FROM public.test_base;
CREATE VIEW public.test_dep_view2 AS SELECT id, value FROM public.test_base;
