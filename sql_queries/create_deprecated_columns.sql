CREATE TABLE IF NOT EXISTS pgvc_deprecated_columns (
    schema_name name NOT NULL,
    view_name name NOT NULL,
    column_name name NOT NULL,
    deprecation_message text,
    removal_date date,
    deprecated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (schema_name, view_name, column_name)
)
