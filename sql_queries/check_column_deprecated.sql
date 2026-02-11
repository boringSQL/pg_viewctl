SELECT
    deprecation_message,
    removal_date::text
FROM pgvc_deprecated_columns
WHERE schema_name = $1
  AND view_name = $2
  AND column_name = $3
