INSERT INTO pgvc_deprecated_columns
    (schema_name, view_name, column_name, deprecation_message, removal_date, deprecated_at)
VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
ON CONFLICT (schema_name, view_name, column_name)
DO UPDATE SET
    deprecation_message = EXCLUDED.deprecation_message,
    removal_date = EXCLUDED.removal_date,
    deprecated_at = CURRENT_TIMESTAMP
