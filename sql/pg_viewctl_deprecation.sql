-- deprecation lifecycle: deprecate_column, undeprecate_column,
-- get_deprecated_columns, check_column_deprecated,
-- pgvc_deprecated_with_dependents view

CREATE EXTENSION pg_viewctl;

CREATE SCHEMA test_depr;

CREATE TABLE test_depr.employees (
    id serial PRIMARY KEY,
    name text NOT NULL,
    salary numeric,
    dept_id integer
);

CREATE VIEW test_depr.employee_summary AS
SELECT id, name, salary FROM test_depr.employees;

CREATE VIEW test_depr.high_earners AS
SELECT id, name, salary FROM test_depr.employee_summary
WHERE salary > 100000;

-- deprecate with message and removal date
SELECT deprecate_column('test_depr', 'employee_summary', 'salary',
                        'Use compensation instead', '2026-06-01');

-- check returns warning with message and date
SELECT check_column_deprecated('test_depr', 'employee_summary', 'salary');

-- non-deprecated column returns NULL
SELECT check_column_deprecated('test_depr', 'employee_summary', 'name');

-- deprecate with NULLs: bare warning, no message/date
SELECT deprecate_column('test_depr', 'employee_summary', 'name', NULL, NULL);
SELECT check_column_deprecated('test_depr', 'employee_summary', 'name');

-- list all deprecated columns (exclude timestamps for stability)
SELECT schema_name, view_name, column_name,
       deprecation_message, removal_date
FROM get_deprecated_columns()
ORDER BY column_name;

-- filter by schema
SELECT schema_name, view_name, column_name
FROM get_deprecated_columns('test_depr')
ORDER BY column_name;

-- nonexistent schema: empty set
SELECT count(*) FROM get_deprecated_columns('no_such_schema');

-- undeprecate
SELECT undeprecate_column('test_depr', 'employee_summary', 'name');
SELECT check_column_deprecated('test_depr', 'employee_summary', 'name');

-- undeprecate already-clean column
SELECT undeprecate_column('test_depr', 'employee_summary', 'name');

-- upsert: update existing deprecation
SELECT deprecate_column('test_depr', 'employee_summary', 'salary',
                        'Updated message', '2026-12-31');

SELECT schema_name, view_name, column_name,
       deprecation_message, removal_date
FROM get_deprecated_columns()
ORDER BY column_name;

-- multiple deprecations across views
SELECT deprecate_column('test_depr', 'high_earners', 'salary',
                        'Deprecated everywhere', NULL);

SELECT schema_name, view_name, column_name, deprecation_message
FROM get_deprecated_columns()
ORDER BY view_name, column_name;

-- deprecated_with_dependents: salary on employee_summary has one dependent
SELECT schema_name, view_name, column_name,
       dependent_count, dependents
FROM pgvc_deprecated_with_dependents
ORDER BY view_name, column_name;

-- error: NULL required args
SAVEPOINT sp1;
SELECT deprecate_column(NULL, 'employee_summary', 'salary');
ROLLBACK TO sp1;

SAVEPOINT sp2;
SELECT deprecate_column('test_depr', NULL, 'salary');
ROLLBACK TO sp2;

SAVEPOINT sp3;
SELECT deprecate_column('test_depr', 'employee_summary', NULL);
ROLLBACK TO sp3;

-- error: nonexistent column
SAVEPOINT sp4;
SELECT deprecate_column('test_depr', 'employee_summary', 'no_such_column');
ROLLBACK TO sp4;

-- error: nonexistent view
SAVEPOINT sp5;
SELECT deprecate_column('test_depr', 'no_such_view', 'salary');
ROLLBACK TO sp5;

-- teardown
DROP SCHEMA test_depr CASCADE;
DROP EXTENSION pg_viewctl CASCADE;
