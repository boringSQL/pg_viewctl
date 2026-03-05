SELECT d.dep_schema, d.dep_view,
       'GRANT ' || g.privilege_type || ' ON ' ||
       quote_ident(d.dep_schema) || '.' || quote_ident(d.dep_view) || ' TO ' ||
       CASE WHEN g.grantee = 'PUBLIC' THEN 'PUBLIC' ELSE quote_ident(g.grantee) END ||
       CASE WHEN g.is_grantable THEN ' WITH GRANT OPTION' ELSE '' END AS grant_sql
FROM pgvc_dependency_order($1, $2) d
CROSS JOIN LATERAL pgvc_get_view_grants(d.dep_schema, d.dep_view) g
ORDER BY d.level, d.dep_schema, d.dep_view
