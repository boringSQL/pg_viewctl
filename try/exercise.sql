-- pg_viewctl exercise
--
-- Setup:
--   cargo pgrx run
--   \i try/schema.sql
--   \i try/seed.sql
--   \i try/exercise.sql

SET search_path TO hr, pg_viewctl, public;


-- 0. what depends on what? ------------------------------------------------

\echo ''
\echo '-- What depends on the employees table?'
SELECT level, dep_schema, dep_view
FROM pgvc_dependency_order('hr', 'employees')
ORDER BY level, dep_view;

\echo ''
\echo '-- Column-level dependencies of employees'
SELECT dependent_view, dependent_column, source_column, dependency_type
FROM get_column_dependencies('hr', 'employees')
ORDER BY source_column, dependent_view;


-- 1. drop column -----------------------------------------------------------
-- Remove middle_name from employees. Only v_employee_details selects it
-- directly, but everything downstream needs to be dropped and recreated too.

\echo ''
\echo '-- 1. DROP COLUMN employees.middle_name'
\echo ''

SELECT * FROM analyze_drop_column('hr', 'employees', 'middle_name');

\echo ''
SELECT step, operation, left(sql, 120) AS sql_preview
FROM generate_drop_column('hr', 'employees', 'middle_name')
ORDER BY step;


-- 2. alter type ------------------------------------------------------------
-- compensation.amount is integer — needs to be numeric(12,2). The change
-- propagates through v_current_compensation, v_department_costs, all the way
-- to v_executive_dashboard, and hits mv_monthly_costs (materialized).

\echo ''
\echo '-- 2. ALTER TYPE compensation.amount -> numeric(12,2)'
\echo ''

SELECT step, operation, left(sql, 120) AS sql_preview
FROM generate_alter_type('hr', 'compensation', 'amount', 'numeric(12,2)')
ORDER BY step;

-- full SQL, ready to pipe into a migration file
\echo ''
SELECT sql
FROM generate_alter_type('hr', 'compensation', 'amount', 'numeric(12,2)')
ORDER BY step;


-- 3. rename view column ----------------------------------------------------
-- v_current_compensation.salary -> annual_compensation. The dependents that
-- reference "salary" get TODO markers; the rest are just recreated as-is.

\echo ''
\echo '-- 3. RENAME v_current_compensation.salary -> annual_compensation'
\echo ''

-- who depends on this view?
SELECT level, dep_schema, dep_view
FROM pgvc_dependency_order('hr', 'v_current_compensation')
ORDER BY level, dep_view;

\echo ''
SELECT sql
FROM generate_rename_view_column(
    'hr', 'v_current_compensation', 'salary', 'annual_compensation'
) ORDER BY step;


-- 4. replace view ----------------------------------------------------------
-- Add tenure_years to v_employee_details. This is the hub of the graph —
-- 5 downstream objects (including a matview) need to be dropped/recreated
-- with their grants preserved.

\echo ''
\echo '-- 4. REPLACE VIEW v_employee_details (add tenure_years)'
\echo ''

SELECT level, dep_schema, dep_view
FROM pgvc_dependency_order('hr', 'v_employee_details')
ORDER BY level, dep_view;

\echo ''
SELECT step, operation
FROM generate_replace_view('hr', 'v_employee_details', $$
SELECT
    e.id AS employee_id,
    e.first_name,
    e.middle_name,
    e.last_name,
    e.full_name,
    e.email,
    e.hire_date,
    e.is_active,
    extract(year FROM age(current_date, e.hire_date))::int AS tenure_years,
    d.id AS department_id,
    d.name AS department_name,
    d.cost_center
FROM employees e
JOIN departments d ON d.id = e.department_id
$$) ORDER BY step;


-- 5. deprecation workflow --------------------------------------------------
-- Mark a column as deprecated before dropping it. Track who still depends
-- on it and when it's safe to remove.

\echo ''
\echo '-- 5. Deprecation workflow'
\echo ''

SELECT deprecate_column('hr', 'employees', 'middle_name',
    'Removal planned for Q3 2026.', '2026-09-01'::date);

SELECT * FROM get_deprecated_columns('hr');

\echo ''
SELECT * FROM pgvc_deprecated_with_dependents;
