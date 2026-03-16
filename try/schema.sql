-- pg_viewctl demo schema — HR analytics
--
-- Dependency graph:
--
--   departments ──┐
--                 ├─► v_employee_details ──┬─► v_department_costs ──┬─► v_executive_dashboard
--   employees ────┘                        │                        │
--                                          ├─► v_employee_utilization ──┘
--   compensation ─► v_current_compensation ┘        ▲
--                          │                        │
--                          ├─► mv_monthly_costs ────┘
--                          │
--   projects ─────► v_project_hours ────────┘
--   time_entries ─┘

DROP SCHEMA IF EXISTS hr CASCADE;
CREATE SCHEMA hr;
SET search_path TO hr;

-- tables -------------------------------------------------------------------

CREATE TABLE departments (
    id          int GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name        text NOT NULL UNIQUE,
    cost_center text NOT NULL,
    budget      numeric(12,2) NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE employees (
    id              int GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    department_id   int NOT NULL REFERENCES departments,
    first_name      text NOT NULL,
    middle_name     text,
    last_name       text NOT NULL,
    full_name       text GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED,
    email           text NOT NULL UNIQUE,
    hire_date       date NOT NULL DEFAULT current_date,
    termination_date date,
    is_active       boolean GENERATED ALWAYS AS (termination_date IS NULL) STORED,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_employees_department ON employees (department_id);
CREATE INDEX idx_employees_active ON employees (is_active) WHERE is_active;

-- amount is integer on purpose — the alter-type exercise fixes this
CREATE TABLE compensation (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    employee_id int NOT NULL REFERENCES employees,
    amount      integer NOT NULL,
    currency    text NOT NULL DEFAULT 'USD',
    effective   date NOT NULL DEFAULT current_date,
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (employee_id, effective)
);

CREATE INDEX idx_compensation_employee ON compensation (employee_id);
CREATE INDEX idx_compensation_effective ON compensation (effective DESC);

CREATE TABLE projects (
    id          int GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name        text NOT NULL,
    department_id int NOT NULL REFERENCES departments,
    is_billable boolean NOT NULL DEFAULT true,
    started_at  date NOT NULL DEFAULT current_date,
    ended_at    date,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE time_entries (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    employee_id int NOT NULL REFERENCES employees,
    project_id  int NOT NULL REFERENCES projects,
    worked_on   date NOT NULL,
    hours       numeric(4,1) NOT NULL CHECK (hours > 0 AND hours <= 24),
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_time_entries_employee ON time_entries (employee_id);
CREATE INDEX idx_time_entries_project ON time_entries (project_id);
CREATE INDEX idx_time_entries_date ON time_entries (worked_on);

-- views on tables ----------------------------------------------------------

CREATE VIEW v_employee_details AS
SELECT
    e.id AS employee_id,
    e.first_name,
    e.middle_name,
    e.last_name,
    e.full_name,
    e.email,
    e.hire_date,
    e.is_active,
    d.id AS department_id,
    d.name AS department_name,
    d.cost_center
FROM employees e
JOIN departments d ON d.id = e.department_id;

CREATE VIEW v_current_compensation AS
SELECT DISTINCT ON (c.employee_id)
    c.employee_id,
    c.amount AS salary,
    c.currency,
    c.effective AS effective_date
FROM compensation c
ORDER BY c.employee_id, c.effective DESC;

CREATE VIEW v_project_hours AS
SELECT
    te.employee_id,
    p.id AS project_id,
    p.name AS project_name,
    p.is_billable,
    sum(te.hours) AS total_hours,
    min(te.worked_on) AS first_entry,
    max(te.worked_on) AS last_entry
FROM time_entries te
JOIN projects p ON p.id = te.project_id
GROUP BY te.employee_id, p.id, p.name, p.is_billable;

-- views on views -----------------------------------------------------------

CREATE VIEW v_department_costs AS
SELECT
    ed.department_id,
    ed.department_name,
    ed.cost_center,
    count(*) AS headcount,
    sum(cc.salary) AS total_salary,
    round(avg(cc.salary), 2) AS avg_salary,
    min(cc.salary) AS min_salary,
    max(cc.salary) AS max_salary
FROM v_employee_details ed
JOIN v_current_compensation cc ON cc.employee_id = ed.employee_id
WHERE ed.is_active
GROUP BY ed.department_id, ed.department_name, ed.cost_center;

CREATE VIEW v_employee_utilization AS
SELECT
    ed.employee_id,
    ed.full_name,
    ed.department_name,
    count(DISTINCT ph.project_id) AS project_count,
    sum(ph.total_hours) AS total_hours,
    sum(ph.total_hours) FILTER (WHERE ph.is_billable) AS billable_hours,
    CASE
        WHEN sum(ph.total_hours) > 0
        THEN round(sum(ph.total_hours) FILTER (WHERE ph.is_billable) / sum(ph.total_hours) * 100, 1)
        ELSE 0
    END AS billable_pct
FROM v_employee_details ed
LEFT JOIN v_project_hours ph ON ph.employee_id = ed.employee_id
WHERE ed.is_active
GROUP BY ed.employee_id, ed.full_name, ed.department_name;

CREATE MATERIALIZED VIEW mv_monthly_costs AS
SELECT
    date_trunc('month', cc.effective_date) AS month,
    ed.department_name,
    count(*) AS changes,
    sum(cc.salary) AS total_salary
FROM v_current_compensation cc
JOIN v_employee_details ed ON ed.employee_id = cc.employee_id
GROUP BY date_trunc('month', cc.effective_date), ed.department_name
ORDER BY month, ed.department_name;

-- view on views-on-views --------------------------------------------------

CREATE VIEW v_executive_dashboard AS
SELECT
    dc.department_id,
    dc.department_name,
    dc.headcount,
    dc.total_salary,
    dc.avg_salary,
    round(coalesce(avg(eu.billable_pct), 0), 1) AS avg_billable_pct,
    coalesce(sum(eu.total_hours), 0) AS total_hours_logged
FROM v_department_costs dc
LEFT JOIN v_employee_utilization eu ON eu.department_name = dc.department_name
GROUP BY dc.department_id, dc.department_name, dc.headcount, dc.total_salary, dc.avg_salary;

-- grants -------------------------------------------------------------------

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hr_analyst') THEN
        CREATE ROLE hr_analyst;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hr_manager') THEN
        CREATE ROLE hr_manager;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hr_executive') THEN
        CREATE ROLE hr_executive;
    END IF;
END $$;

GRANT USAGE ON SCHEMA hr TO hr_analyst, hr_manager, hr_executive;
GRANT SELECT ON v_project_hours, v_employee_utilization TO hr_analyst;
GRANT SELECT ON v_employee_details, v_current_compensation, v_department_costs TO hr_manager;
GRANT SELECT ON
    v_employee_details, v_current_compensation, v_project_hours,
    v_department_costs, v_employee_utilization, mv_monthly_costs,
    v_executive_dashboard
TO hr_executive;
