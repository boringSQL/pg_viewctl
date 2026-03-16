SET search_path TO hr;

BEGIN;

INSERT INTO departments (name, cost_center, budget) VALUES
    ('Engineering',  'ENG-100', 2500000),
    ('Product',      'PRD-200',  800000),
    ('Design',       'DSN-300',  600000),
    ('Data Science', 'DAT-400',  900000),
    ('Sales',        'SAL-500', 1200000),
    ('Operations',   'OPS-600',  700000);

INSERT INTO employees (department_id, first_name, middle_name, last_name, email, hire_date) VALUES
    (1, 'Tomasz',   NULL,   'Kowalski',  'tomasz.kowalski@example.com',  '2020-03-15'),
    (1, 'Priya',    'R',    'Sharma',    'priya.sharma@example.com',     '2020-06-01'),
    (1, 'Luis',     NULL,   'Herrera',   'luis.herrera@example.com',     '2021-01-10'),
    (1, 'Mei',      NULL,   'Zhang',     'mei.zhang@example.com',       '2021-09-20'),
    (1, 'Oleg',     'A',    'Volkov',    'oleg.volkov@example.com',     '2022-02-14'),
    (1, 'Sara',     NULL,   'Lindqvist', 'sara.lindqvist@example.com',  '2022-07-01'),
    (1, 'James',    'K',    'Osei',      'james.osei@example.com',      '2023-03-01'),
    (1, 'Katarina', NULL,   'Novak',     'katarina.novak@example.com',  '2023-11-15'),
    (2, 'Raj',      NULL,   'Patel',     'raj.patel@example.com',       '2020-04-01'),
    (2, 'Emily',    'J',    'Fraser',    'emily.fraser@example.com',    '2021-02-15'),
    (2, 'Yusuf',    NULL,   'Demir',     'yusuf.demir@example.com',     '2021-08-01'),
    (2, 'Hannah',   NULL,   'Berger',    'hannah.berger@example.com',   '2022-05-15'),
    (2, 'Chen',     'W',    'Li',        'chen.li@example.com',         '2023-01-10'),
    (3, 'Ingrid',   NULL,   'Haugen',    'ingrid.haugen@example.com',   '2020-07-01'),
    (3, 'Marco',    'T',    'Rossi',     'marco.rossi@example.com',     '2021-04-15'),
    (3, 'Aisha',    NULL,   'Okafor',    'aisha.okafor@example.com',    '2022-03-01'),
    (3, 'Finn',     NULL,   'McCarthy',  'finn.mccarthy@example.com',   '2023-06-15'),
    (4, 'Naomi',    'L',    'Tanaka',    'naomi.tanaka@example.com',    '2020-09-01'),
    (4, 'Dmitri',   NULL,   'Petrov',    'dmitri.petrov@example.com',   '2021-06-01'),
    (4, 'Fatima',   NULL,   'Al-Rashid', 'fatima.alrashid@example.com', '2022-01-15'),
    (4, 'Erik',     'S',    'Johansson', 'erik.johansson@example.com',  '2022-10-01'),
    (4, 'Lucia',    NULL,   'Moreno',    'lucia.moreno@example.com',    '2023-08-01'),
    (5, 'Gabriel',  NULL,   'Santos',    'gabriel.santos@example.com',  '2020-05-01'),
    (5, 'Chloe',    'M',    'Dubois',    'chloe.dubois@example.com',    '2021-03-15'),
    (5, 'Kenji',    NULL,   'Watanabe',  'kenji.watanabe@example.com',  '2021-11-01'),
    (5, 'Amara',    NULL,   'Diop',      'amara.diop@example.com',      '2022-06-15'),
    (5, 'Pavel',    'D',    'Horvat',    'pavel.horvat@example.com',    '2023-04-01'),
    (6, 'Signe',    NULL,   'Larsen',    'signe.larsen@example.com',    '2020-08-01'),
    (6, 'Oscar',    'F',    'Reyes',     'oscar.reyes@example.com',     '2021-07-15'),
    (6, 'Nadia',    NULL,   'Khoury',    'nadia.khoury@example.com',    '2022-09-01');

UPDATE employees SET termination_date = '2024-06-30'
WHERE email = 'sara.lindqvist@example.com';

-- initial salaries
INSERT INTO compensation (employee_id, amount, effective)
SELECT id,
    CASE department_id
        WHEN 1 THEN 95000 + (id * 3000 % 30000)
        WHEN 2 THEN 85000 + (id * 2500 % 20000)
        WHEN 3 THEN 80000 + (id * 2000 % 15000)
        WHEN 4 THEN 100000 + (id * 3500 % 25000)
        WHEN 5 THEN 70000 + (id * 4000 % 30000)
        WHEN 6 THEN 75000 + (id * 2000 % 15000)
    END,
    hire_date
FROM employees;

-- raises for people hired before 2022
INSERT INTO compensation (employee_id, amount, effective)
SELECT
    c.employee_id,
    c.amount + (c.amount * 0.05)::int,
    c.effective + interval '1 year'
FROM compensation c
JOIN employees e ON e.id = c.employee_id
WHERE e.hire_date < '2022-01-01'
AND NOT EXISTS (
    SELECT 1 FROM compensation c2
    WHERE c2.employee_id = c.employee_id
    AND c2.effective > c.effective
    AND c2.effective <= c.effective + interval '1 year'
);

INSERT INTO projects (name, department_id, is_billable, started_at, ended_at) VALUES
    ('Platform Rewrite',     1, true,  '2023-01-15', NULL),
    ('Mobile App v2',        1, true,  '2023-06-01', NULL),
    ('Internal Tooling',     1, false, '2023-03-01', '2024-02-28'),
    ('Q4 Launch Campaign',   2, true,  '2023-09-01', '2023-12-31'),
    ('Design System',        3, false, '2023-04-01', NULL),
    ('Brand Refresh',        3, true,  '2023-10-01', '2024-03-31'),
    ('ML Pipeline',          4, true,  '2023-02-01', NULL),
    ('Customer Churn Model', 4, true,  '2023-08-15', NULL),
    ('Sales Dashboard',      5, false, '2023-05-01', NULL),
    ('SOC2 Compliance',      6, false, '2023-07-01', NULL);

INSERT INTO time_entries (employee_id, project_id, worked_on, hours)
SELECT
    e.id,
    p.id,
    d::date,
    round((3 + random() * 5)::numeric, 1)
FROM employees e
JOIN projects p ON p.department_id = e.department_id
CROSS JOIN generate_series('2023-06-01'::date, '2024-06-01'::date, '14 days'::interval) d
WHERE e.is_active
AND d::date >= p.started_at
AND d::date <= coalesce(p.ended_at, '2024-06-01'::date)
AND random() < 0.6;

REFRESH MATERIALIZED VIEW mv_monthly_costs;

COMMIT;

-- sanity check
\echo ''
\echo 'Row counts:'
SELECT 'departments'    AS tbl, count(*) FROM hr.departments
UNION ALL SELECT 'employees', count(*) FROM hr.employees
UNION ALL SELECT 'compensation', count(*) FROM hr.compensation
UNION ALL SELECT 'projects', count(*) FROM hr.projects
UNION ALL SELECT 'time_entries', count(*) FROM hr.time_entries
ORDER BY tbl;

\echo ''
\echo 'Executive dashboard:'
SELECT * FROM hr.v_executive_dashboard ORDER BY department_name;
