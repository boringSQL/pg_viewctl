-- dependency analysis: get_column_dependencies, analyze_drop_column,
-- pgvc_column_dependencies view, pgvc_analyze_drop_report

CREATE EXTENSION pg_viewctl;

CREATE SCHEMA test_deps;

CREATE TABLE test_deps.employees (
    id serial PRIMARY KEY,
    name text NOT NULL,
    salary numeric,
    dept_id integer
);

CREATE TABLE test_deps.departments (
    id serial PRIMARY KEY,
    name text NOT NULL
);

-- three-layer view stack: join → filter, aggregate
CREATE VIEW test_deps.employee_summary AS
SELECT e.id, e.name, e.salary, d.name AS department
FROM test_deps.employees e
JOIN test_deps.departments d ON d.id = e.dept_id;

CREATE VIEW test_deps.high_earners AS
SELECT id, name, salary, department
FROM test_deps.employee_summary
WHERE salary > 100000;

CREATE VIEW test_deps.payroll_summary AS
SELECT department, COUNT(*) AS headcount, SUM(salary) AS total_salary
FROM test_deps.employee_summary
GROUP BY department;

-- materialized view on base table
CREATE MATERIALIZED VIEW test_deps.dept_stats AS
SELECT d.id, d.name, COUNT(e.id) AS emp_count
FROM test_deps.departments d
LEFT JOIN test_deps.employees e ON e.dept_id = d.id
GROUP BY d.id, d.name;

-- direct deps from base table: only employee_summary sees it
SELECT dependent_schema, dependent_view, dependent_column, source_column
FROM get_column_dependencies('test_deps', 'employees')
ORDER BY source_column, dependent_view;

-- deps from intermediate view: high_earners and payroll_summary
SELECT dependent_schema, dependent_view, dependent_column, source_column
FROM get_column_dependencies('test_deps', 'employee_summary')
ORDER BY dependent_view, source_column;

-- leaf view: nothing depends on it
SELECT dependent_view
FROM get_column_dependencies('test_deps', 'high_earners');

-- departments.name → employee_summary.department is a rename,
-- so dependent_column falls back to source name
SELECT dependent_view, dependent_column, source_column
FROM get_column_dependencies('test_deps', 'departments')
ORDER BY dependent_view, source_column;

-- matview deps from departments
SELECT dependent_view, dependent_column, source_column
FROM get_column_dependencies('test_deps', 'departments')
WHERE dependent_view = 'dept_stats'
ORDER BY source_column;

-- analyze_drop_column: salary used by two downstream views
SELECT dependent_view, dependent_column, impact_severity, usage_location
FROM analyze_drop_column('test_deps', 'employee_summary', 'salary')
ORDER BY dependent_view;

-- analyze_drop_column: leaf view, nothing breaks
SELECT dependent_view
FROM analyze_drop_column('test_deps', 'high_earners', 'salary');

-- report format
SELECT pgvc_analyze_drop_report('test_deps', 'employee_summary', 'salary');
SELECT pgvc_analyze_drop_report('test_deps', 'high_earners', 'salary');

-- pgvc_column_dependencies view
SELECT source_object, source_column, dependent_view, dependent_column,
       dependency_type, impact_severity
FROM pgvc_column_dependencies
WHERE source_schema = 'test_deps' AND source_object = 'employees'
ORDER BY source_column, dependent_view;

-- cross-schema dependency
CREATE SCHEMA test_deps_xref;

CREATE VIEW test_deps_xref.external_view AS
SELECT id, name FROM test_deps.employees;

SELECT dependent_schema, dependent_view, source_column
FROM get_column_dependencies('test_deps', 'employees')
WHERE dependent_schema = 'test_deps_xref'
ORDER BY source_column;

-- nonexistent object/schema: empty sets, no errors
SELECT count(*) FROM get_column_dependencies('test_deps', 'no_such_table');
SELECT count(*) FROM get_column_dependencies('no_such_schema', 'employees');

-- pgvc_dependency_order: full graph from base table
-- employees at 0, employee_summary + dept_stats + external_view at 1,
-- high_earners + payroll_summary at 2
SELECT * FROM pgvc_dependency_order('test_deps', 'employees');

-- pgvc_dependency_order: from intermediate view
SELECT * FROM pgvc_dependency_order('test_deps', 'employee_summary');

-- pgvc_dependency_order: leaf view (nothing depends on it)
SELECT * FROM pgvc_dependency_order('test_deps', 'high_earners');

-- pgvc_dependency_order: diamond dependency
CREATE VIEW test_deps.combined_report AS
SELECT h.name, p.total_salary
FROM test_deps.high_earners h
JOIN test_deps.payroll_summary p ON h.department = p.department;

SELECT * FROM pgvc_dependency_order('test_deps', 'employee_summary');

-- pgvc_dependency_order: cross-schema (external_view at level 1)
SELECT dep_schema, dep_view
FROM pgvc_dependency_order('test_deps', 'employees')
WHERE dep_schema = 'test_deps_xref';

-- pgvc_dependency_order: nonexistent object returns empty set
SELECT * FROM pgvc_dependency_order('test_deps', 'no_such_table');

-- pgvc_dependency_order: nonexistent schema returns empty set
SELECT * FROM pgvc_dependency_order('no_such_schema', 'employees');

-- teardown
DROP SCHEMA test_deps_xref CASCADE;
DROP SCHEMA test_deps CASCADE;
DROP EXTENSION pg_viewctl CASCADE;
