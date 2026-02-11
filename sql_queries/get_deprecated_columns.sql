SELECT schema_name::text,
       view_name::text,
       column_name::text,
       deprecation_message,
       removal_date::text,
       deprecated_at::text
FROM pgvc_deprecated_columns
WHERE ($1 IS NULL OR schema_name = $1)
ORDER BY schema_name, view_name, column_name
