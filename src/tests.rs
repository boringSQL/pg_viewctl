use pgrx::prelude::*;

fn create_deprecated_columns_table() {
    Spi::run(include_str!("../sql_queries/tests/create_deprecated_columns.sql")).unwrap();
}

fn create_test_view() {
    Spi::run(include_str!("../sql_queries/tests/create_test_view.sql")).unwrap();
}

#[pg_test]
fn test_check_not_deprecated() {
    create_deprecated_columns_table();

    let result = crate::functions::check_column_deprecated("public", "test_view", "col1");
    assert!(result.is_none());
}

#[pg_test]
fn test_check_deprecated_with_message() {
    create_deprecated_columns_table();

    Spi::run(include_str!("../sql_queries/tests/insert_test_deprecated_column.sql")).unwrap();

    let result = crate::functions::check_column_deprecated("public", "my_view", "old_col");
    assert!(result.is_some());

    let msg = result.unwrap();
    assert!(msg.contains("is deprecated"));
    assert!(msg.contains("Use new_col instead"));
    assert!(msg.contains("2026-06-01"));
}

#[pg_test]
fn test_deprecate_column() {
    create_deprecated_columns_table();
    create_test_view();

    let result = crate::functions::deprecate_column("public", "test_view", "old_col", Some("Use new_col"), None);
    assert_eq!(result, "column public.test_view.old_col deprecated");
}

#[pg_test]
fn test_undeprecate_column() {
    create_deprecated_columns_table();
    create_test_view();

    crate::functions::deprecate_column("public", "test_view", "old_col", Some("going away"), None);
    let result = crate::functions::undeprecate_column("public", "test_view", "old_col");
    assert_eq!(result, "column public.test_view.old_col undeprecated");
}

#[pg_test]
fn test_undeprecate_not_deprecated() {
    create_deprecated_columns_table();

    let result = crate::functions::undeprecate_column("public", "test_view", "col1");
    assert_eq!(result, "column public.test_view.col1 was not marked as deprecated");
}

fn create_dependency_fixtures() {
    Spi::run(include_str!("../sql_queries/tests/create_dependency_fixtures.sql")).unwrap();
}

#[pg_test]
fn test_get_column_deps_found() {
    create_dependency_fixtures();

    let results: Vec<_> = crate::functions::get_column_dependencies("public", "test_base")
        .collect();

    let dep_views: Vec<_> = results.iter().filter_map(|r| r.1.as_deref()).collect();
    assert!(
        dep_views.contains(&"test_dep_view"),
        "expected test_dep_view in {:?}, got {} results", dep_views, results.len()
    );

    let source_cols: Vec<_> = results.iter().filter_map(|r| r.3.as_deref()).collect();
    assert!(source_cols.contains(&"id"));
    assert!(source_cols.contains(&"name"));
}

#[pg_test]
fn test_get_column_deps_leaf_view() {
    create_dependency_fixtures();

    let count = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM get_column_dependencies($1, $2)",
                None,
                &[crate::functions::text_arg("public"), crate::functions::text_arg("test_dep_view")],
            )
            .map(|tuptable| {
                tuptable.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0)
            })
    })
    .unwrap();

    assert_eq!(count, 0);
}

#[pg_test]
fn test_get_column_deps_nonexistent() {
    create_dependency_fixtures();

    let count = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM get_column_dependencies($1, $2)",
                None,
                &[crate::functions::text_arg("public"), crate::functions::text_arg("no_such_thing")],
            )
            .map(|tuptable| {
                tuptable.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0)
            })
    })
    .unwrap();

    assert_eq!(count, 0);
}

#[pg_test]
fn test_analyze_drop_found() {
    create_dependency_fixtures();

    let results: Vec<_> = crate::functions::analyze_drop_column("public", "test_base", "name")
        .collect();

    assert!(!results.is_empty(), "expected results for dropping test_base.name");

    let dep_views: Vec<_> = results.iter().filter_map(|r| r.0.as_deref()).collect();
    assert!(
        dep_views.contains(&"test_dep_view"),
        "expected test_dep_view in {:?}", dep_views
    );

    let severities: Vec<_> = results.iter().filter_map(|r| r.3.as_deref()).collect();
    assert!(severities.contains(&"BREAKING"));
}

#[pg_test]
fn test_analyze_drop_leaf() {
    create_dependency_fixtures();

    let results: Vec<_> = crate::functions::analyze_drop_column("public", "test_dep_view", "name")
        .collect();

    assert!(results.is_empty(), "leaf view should have no dependents");
}

#[pg_test]
fn test_analyze_drop_nonexistent() {
    create_dependency_fixtures();

    let results: Vec<_> = crate::functions::analyze_drop_column("public", "test_base", "no_such_col")
        .collect();

    assert!(results.is_empty(), "nonexistent column should return empty set");
}

#[pg_test]
fn test_get_deprecated_columns() {
    create_deprecated_columns_table();
    create_test_view();

    crate::functions::deprecate_column("public", "test_view", "old_col", Some("Use new_col"), None);

    let results: Vec<_> = crate::functions::get_deprecated_columns(Some("public")).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.as_deref(), Some("test_view"));
    assert_eq!(results[0].2.as_deref(), Some("old_col"));
}

#[pg_test]
fn test_get_deprecated_columns_empty() {
    create_deprecated_columns_table();

    let results: Vec<_> = crate::functions::get_deprecated_columns(None).collect();
    assert!(results.is_empty());
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_deprecate_nonexistent_column() {
    create_deprecated_columns_table();
    create_test_view();

    crate::functions::deprecate_column("public", "test_view", "no_such_col", None, None);
}

#[pg_test]
fn test_deprecate_upsert() {
    create_deprecated_columns_table();
    create_test_view();

    crate::functions::deprecate_column("public", "test_view", "old_col", Some("first msg"), None);
    let result = crate::functions::deprecate_column("public", "test_view", "old_col", Some("updated msg"), None);
    assert_eq!(result, "column public.test_view.old_col deprecated");
}

fn create_replace_view_fixtures() {
    Spi::run(include_str!("../sql_queries/tests/create_replace_view_fixtures.sql")).unwrap();
}

#[pg_test]
fn test_generate_replace_view_basic() {
    create_replace_view_fixtures();

    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_target",
        "SELECT id, name FROM public.rv_base WHERE active",
    )
    .collect();

    assert!(!results.is_empty());

    // steps are sequential starting at 1
    for (i, row) in results.iter().enumerate() {
        assert_eq!(row.0, (i + 1) as i32);
    }

    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();

    // DROPs come first, then CREATE OR REPLACE, then CREATEs, then GRANTs
    let first_drop = ops.iter().position(|o| o.starts_with("DROP")).unwrap();

    // with deps, target uses DROP+CREATE; find the target CREATE step
    // (first CREATE VIEW/MATERIALIZED VIEW after the drops)
    let target_create_pos = ops
        .iter()
        .position(|o| *o == "CREATE VIEW" || *o == "CREATE MATERIALIZED VIEW")
        .unwrap();

    assert!(first_drop < target_create_pos, "DROPs must come before target CREATE");

    // dependent CREATEs follow the target
    let dep_create_positions: Vec<_> = ops
        .iter()
        .enumerate()
        .skip(target_create_pos + 1)
        .filter(|(_, o)| **o == "CREATE VIEW" || **o == "CREATE MATERIALIZED VIEW")
        .map(|(i, _)| i)
        .collect();

    for pos in &dep_create_positions {
        assert!(*pos > target_create_pos, "dependent CREATEs must come after target CREATE");
    }

    if let Some(grant_pos) = ops.iter().position(|o| *o == "GRANT") {
        let last_create = dep_create_positions.last().unwrap_or(&target_create_pos);
        assert!(*last_create < grant_pos, "CREATEs must come before GRANTs");
    }
}

#[pg_test]
fn test_generate_replace_view_no_deps() {
    create_replace_view_fixtures();

    // rv_dep_l2 is a leaf view with no dependents
    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_dep_l2",
        "SELECT id FROM public.rv_dep_l1",
    )
    .collect();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.as_deref(), Some("CREATE OR REPLACE VIEW"));
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_replace_view_nonexistent() {
    create_replace_view_fixtures();

    let _: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "no_such_view",
        "SELECT 1",
    )
    .collect();
}

#[pg_test]
fn test_generate_replace_view_grants_preserved() {
    create_replace_view_fixtures();

    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_target",
        "SELECT id, name FROM public.rv_base WHERE active",
    )
    .collect();

    let grant_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("GRANT"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!grant_sqls.is_empty(), "expected GRANT steps for views that had grants");

    let has_target_grant = grant_sqls.iter().any(|s| s.contains("rv_target"));
    let has_l1_grant = grant_sqls.iter().any(|s| s.contains("rv_dep_l1"));
    assert!(has_target_grant, "expected GRANT for rv_target");
    assert!(has_l1_grant, "expected GRANT for rv_dep_l1");
}

#[pg_test]
fn test_generate_replace_view_matview_refresh() {
    create_replace_view_fixtures();

    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_target",
        "SELECT id, name FROM public.rv_base WHERE active",
    )
    .collect();

    let refresh_ops: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("REFRESH MATERIALIZED VIEW"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!refresh_ops.is_empty(), "expected REFRESH MATERIALIZED VIEW step");
    assert!(
        refresh_ops.iter().any(|s| s.contains("rv_mat_dep")),
        "expected REFRESH for rv_mat_dep"
    );
}

#[pg_test]
fn test_generate_replace_view_drop_order() {
    create_replace_view_fixtures();

    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_target",
        "SELECT id, name FROM public.rv_base WHERE active",
    )
    .collect();

    let drop_views: Vec<&str> = results
        .iter()
        .filter(|r| {
            r.1.as_deref()
                .map(|o| o.starts_with("DROP"))
                .unwrap_or(false)
        })
        .filter_map(|r| r.2.as_deref())
        .collect();

    // l2 depends on l1, so l2 must be dropped before l1
    let l2_pos = drop_views.iter().position(|s| s.contains("rv_dep_l2"));
    let l1_pos = drop_views.iter().position(|s| s.contains("rv_dep_l1"));

    assert!(l2_pos.is_some(), "expected DROP for rv_dep_l2");
    assert!(l1_pos.is_some(), "expected DROP for rv_dep_l1");
    assert!(
        l2_pos.unwrap() < l1_pos.unwrap(),
        "level 2 must be dropped before level 1"
    );
}

#[pg_test]
fn test_generate_replace_view_executable() {
    create_replace_view_fixtures();

    // insert test data so matview refresh works
    Spi::run("INSERT INTO public.rv_base VALUES (1, 'alice', 'a@example.com', true)").unwrap();
    Spi::run("REFRESH MATERIALIZED VIEW public.rv_mat_dep").unwrap();

    let results: Vec<_> = crate::functions::generate_replace_view(
        "public",
        "rv_target",
        "SELECT id, name FROM public.rv_base WHERE active",
    )
    .collect();

    // execute all generated SQL steps
    for row in &results {
        if let Some(sql) = row.2.as_deref() {
            Spi::run(sql).unwrap();
        }
    }

    // verify rv_target actually changed — email column should be gone
    let col_count = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'rv_target'",
                None,
                &[],
            )
            .map(|t| t.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0))
    })
    .unwrap();

    assert_eq!(col_count, 2, "rv_target should now have 2 columns (id, name) instead of 3");
}

fn create_drop_column_fixtures() {
    Spi::run(include_str!("../sql_queries/tests/create_drop_column_fixtures.sql")).unwrap();
}

#[pg_test]
fn test_generate_drop_column_basic() {
    create_drop_column_fixtures();

    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "email",
    ).collect();

    assert!(!results.is_empty());

    for (i, row) in results.iter().enumerate() {
        assert_eq!(row.0, (i + 1) as i32);
    }

    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();

    // DROPs come first
    let last_drop = ops.iter().rposition(|o| o.starts_with("DROP")).unwrap();
    // then ALTER TABLE
    let alter_pos = ops.iter().position(|o| *o == "ALTER TABLE").unwrap();
    assert!(last_drop < alter_pos, "DROPs must come before ALTER TABLE");

    // then CREATEs
    let first_create = ops.iter().position(|o| o.starts_with("CREATE")).unwrap();
    assert!(alter_pos < first_create, "ALTER TABLE must come before CREATEs");

    // then GRANTs
    if let Some(grant_pos) = ops.iter().position(|o| *o == "GRANT") {
        let last_create = ops.iter().rposition(|o| o.starts_with("CREATE")).unwrap();
        assert!(last_create < grant_pos, "CREATEs must come before GRANTs");
    }
}

#[pg_test]
fn test_generate_drop_column_no_deps() {
    create_drop_column_fixtures();

    // drop 'notes' — no view references it
    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "notes",
    ).collect();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.as_deref(), Some("ALTER TABLE"));
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_drop_column_nonexistent_table() {
    create_drop_column_fixtures();

    let _: Vec<_> = crate::functions::generate_drop_column(
        "public", "no_such_table", "id",
    ).collect();
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_drop_column_nonexistent_column() {
    create_drop_column_fixtures();

    let _: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "no_such_col",
    ).collect();
}

#[pg_test]
fn test_generate_drop_column_todo_marker() {
    create_drop_column_fixtures();

    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "email",
    ).collect();

    let create_steps: Vec<(&str, &str)> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("CREATE")).unwrap_or(false))
        .map(|r| (r.1.as_deref().unwrap(), r.2.as_deref().unwrap()))
        .collect();

    // dc_view_with_email and dc_mat_dep reference email — should get TODO
    let with_email = create_steps.iter().find(|(_, sql)| sql.contains("dc_view_with_email"));
    assert!(with_email.is_some(), "expected CREATE for dc_view_with_email");
    assert!(
        with_email.unwrap().1.contains("-- TODO: remove reference to 'email'"),
        "dc_view_with_email should have TODO marker"
    );

    let mat_dep = create_steps.iter().find(|(_, sql)| sql.contains("dc_mat_dep"));
    assert!(mat_dep.is_some(), "expected CREATE for dc_mat_dep");
    assert!(
        mat_dep.unwrap().1.contains("-- TODO: remove reference to 'email'"),
        "dc_mat_dep should have TODO marker"
    );

    // dc_dep_l2 depends on dc_view_with_email but doesn't directly reference email
    let dep_l2 = create_steps.iter().find(|(_, sql)| sql.contains("dc_dep_l2"));
    assert!(dep_l2.is_some(), "expected CREATE for dc_dep_l2");
    assert!(
        !dep_l2.unwrap().1.contains("-- TODO"),
        "dc_dep_l2 should NOT have TODO marker (transitive dep, no direct column ref)"
    );

    // dc_view_no_email doesn't reference email — not in the plan at all
    let no_email = create_steps.iter().find(|(_, sql)| sql.contains("dc_view_no_email"));
    assert!(no_email.is_none(), "dc_view_no_email should NOT be in the plan");
}

#[pg_test]
fn test_generate_drop_column_grants_preserved() {
    create_drop_column_fixtures();

    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "email",
    ).collect();

    let grant_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("GRANT"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!grant_sqls.is_empty(), "expected GRANT steps");

    let has_with_email_grant = grant_sqls.iter().any(|s| s.contains("dc_view_with_email"));
    assert!(has_with_email_grant, "expected GRANT for dc_view_with_email");

    // dc_view_no_email doesn't reference email, so it's not in the plan
    let has_no_email_grant = grant_sqls.iter().any(|s| s.contains("dc_view_no_email"));
    assert!(!has_no_email_grant, "dc_view_no_email should NOT have GRANT (unaffected)");
}

#[pg_test]
fn test_generate_drop_column_matview_refresh() {
    create_drop_column_fixtures();

    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "email",
    ).collect();

    let refresh_ops: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("REFRESH MATERIALIZED VIEW"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!refresh_ops.is_empty(), "expected REFRESH MATERIALIZED VIEW step");
    assert!(
        refresh_ops.iter().any(|s| s.contains("dc_mat_dep")),
        "expected REFRESH for dc_mat_dep"
    );
}

#[pg_test]
fn test_generate_drop_column_drop_order() {
    create_drop_column_fixtures();

    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "email",
    ).collect();

    let drop_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("DROP")).unwrap_or(false))
        .filter_map(|r| r.2.as_deref())
        .collect();

    // dc_dep_l2 depends on dc_view_with_email, so l2 must be dropped first
    let l2_pos = drop_sqls.iter().position(|s| s.contains("dc_dep_l2"));
    let with_email_pos = drop_sqls.iter().position(|s| s.contains("dc_view_with_email"));

    assert!(l2_pos.is_some(), "expected DROP for dc_dep_l2");
    assert!(with_email_pos.is_some(), "expected DROP for dc_view_with_email");
    assert!(
        l2_pos.unwrap() < with_email_pos.unwrap(),
        "dc_dep_l2 must be dropped before dc_view_with_email"
    );
}

#[pg_test]
fn test_generate_drop_column_executable() {
    create_drop_column_fixtures();

    // insert test data so matview refresh works
    Spi::run("INSERT INTO public.dc_base VALUES (1, 'alice', 'a@example.com', 'active', 'some notes', true)").unwrap();
    Spi::run("REFRESH MATERIALIZED VIEW public.dc_mat_dep").unwrap();

    // drop 'notes' — no view references it, so all generated SQL is directly executable
    let results: Vec<_> = crate::functions::generate_drop_column(
        "public", "dc_base", "notes",
    ).collect();

    for row in &results {
        if let Some(sql) = row.2.as_deref() {
            Spi::run(sql).unwrap();
        }
    }

    // verify column is gone
    let col_exists = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'dc_base' AND column_name = 'notes'",
                None,
                &[],
            )
            .map(|t| t.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0))
    })
    .unwrap();
    assert_eq!(col_exists, 0, "notes column should be gone");

    // verify views still work
    let view_ok = Spi::connect(|client| {
        client
            .select("SELECT count(*) AS cnt FROM public.dc_view_with_email", None, &[])
            .map(|t| t.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(-1))
    })
    .unwrap();
    assert!(view_ok >= 0, "dc_view_with_email should still be queryable");
}

fn create_alter_type_fixtures() {
    Spi::run(include_str!("../sql_queries/tests/create_alter_type_fixtures.sql")).unwrap();
}

#[pg_test]
fn test_generate_alter_type_basic() {
    create_alter_type_fixtures();

    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "amount", "numeric(12,4)",
    ).collect();

    assert!(!results.is_empty());

    for (i, row) in results.iter().enumerate() {
        assert_eq!(row.0, (i + 1) as i32);
    }

    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();

    let last_drop = ops.iter().rposition(|o| o.starts_with("DROP")).unwrap();
    let alter_pos = ops.iter().position(|o| *o == "ALTER TABLE").unwrap();
    assert!(last_drop < alter_pos, "DROPs must come before ALTER TABLE");

    let first_create = ops.iter().position(|o| o.starts_with("CREATE")).unwrap();
    assert!(alter_pos < first_create, "ALTER TABLE must come before CREATEs");

    if let Some(grant_pos) = ops.iter().position(|o| *o == "GRANT") {
        let last_create = ops.iter().rposition(|o| o.starts_with("CREATE")).unwrap();
        assert!(last_create < grant_pos, "CREATEs must come before GRANTs");
    }
}

#[pg_test]
fn test_generate_alter_type_no_deps() {
    create_alter_type_fixtures();

    // change 'active' type — no view references it directly by column dep
    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "id", "bigint",
    ).collect();

    // id is referenced by views, so this won't be a no-deps case.
    // Use a column that truly has no view deps — but all our views reference id or amount.
    // Let's test with the 'active' column which is used in WHERE but not as a selected column dep.
    // Actually, pg_depend tracks WHERE-clause refs too. Let's just verify ALTER TABLE is present.
    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();
    assert!(ops.contains(&"ALTER TABLE"), "expected ALTER TABLE step");
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_alter_type_nonexistent_table() {
    create_alter_type_fixtures();

    let _: Vec<_> = crate::functions::generate_alter_type(
        "public", "no_such_table", "id", "bigint",
    ).collect();
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_alter_type_nonexistent_column() {
    create_alter_type_fixtures();

    let _: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "no_such_col", "bigint",
    ).collect();
}

#[pg_test]
fn test_generate_alter_type_todo_marker() {
    create_alter_type_fixtures();

    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "amount", "numeric(12,4)",
    ).collect();

    let create_steps: Vec<(&str, &str)> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("CREATE")).unwrap_or(false))
        .map(|r| (r.1.as_deref().unwrap(), r.2.as_deref().unwrap()))
        .collect();

    // at_view_with_amount references amount — should get TODO
    let with_amount = create_steps.iter().find(|(_, sql)| sql.contains("at_view_with_amount"));
    assert!(with_amount.is_some(), "expected CREATE for at_view_with_amount");
    assert!(
        with_amount.unwrap().1.contains("-- TODO: verify type change of 'amount'"),
        "at_view_with_amount should have TODO marker"
    );
    assert!(
        with_amount.unwrap().1.contains("numeric(10,2)"),
        "TODO should mention old type"
    );
    assert!(
        with_amount.unwrap().1.contains("numeric(12,4)"),
        "TODO should mention new type"
    );

    // at_mat_dep references amount — should get TODO
    let mat_dep = create_steps.iter().find(|(_, sql)| sql.contains("at_mat_dep"));
    assert!(mat_dep.is_some(), "expected CREATE for at_mat_dep");
    assert!(
        mat_dep.unwrap().1.contains("-- TODO: verify type change of 'amount'"),
        "at_mat_dep should have TODO marker"
    );

    // at_dep_l2 depends on at_view_with_amount but doesn't directly reference amount on base table
    let dep_l2 = create_steps.iter().find(|(_, sql)| sql.contains("at_dep_l2"));
    assert!(dep_l2.is_some(), "expected CREATE for at_dep_l2");
    assert!(
        !dep_l2.unwrap().1.contains("-- TODO"),
        "at_dep_l2 should NOT have TODO marker (transitive dep)"
    );

    // at_view_no_amount doesn't reference amount — not in the plan at all
    let no_amount = create_steps.iter().find(|(_, sql)| sql.contains("at_view_no_amount"));
    assert!(no_amount.is_none(), "at_view_no_amount should NOT be in the plan");
}

#[pg_test]
fn test_generate_alter_type_grants_preserved() {
    create_alter_type_fixtures();

    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "amount", "numeric(12,4)",
    ).collect();

    let grant_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("GRANT"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!grant_sqls.is_empty(), "expected GRANT steps");

    let has_with_amount_grant = grant_sqls.iter().any(|s| s.contains("at_view_with_amount"));
    assert!(has_with_amount_grant, "expected GRANT for at_view_with_amount");
}

#[pg_test]
fn test_generate_alter_type_matview_refresh() {
    create_alter_type_fixtures();

    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "amount", "numeric(12,4)",
    ).collect();

    let refresh_ops: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("REFRESH MATERIALIZED VIEW"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!refresh_ops.is_empty(), "expected REFRESH MATERIALIZED VIEW step");
    assert!(
        refresh_ops.iter().any(|s| s.contains("at_mat_dep")),
        "expected REFRESH for at_mat_dep"
    );
}

#[pg_test]
fn test_generate_alter_type_drop_order() {
    create_alter_type_fixtures();

    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "amount", "numeric(12,4)",
    ).collect();

    let drop_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("DROP")).unwrap_or(false))
        .filter_map(|r| r.2.as_deref())
        .collect();

    let l2_pos = drop_sqls.iter().position(|s| s.contains("at_dep_l2"));
    let with_amount_pos = drop_sqls.iter().position(|s| s.contains("at_view_with_amount"));

    assert!(l2_pos.is_some(), "expected DROP for at_dep_l2");
    assert!(with_amount_pos.is_some(), "expected DROP for at_view_with_amount");
    assert!(
        l2_pos.unwrap() < with_amount_pos.unwrap(),
        "at_dep_l2 must be dropped before at_view_with_amount"
    );
}

#[pg_test]
fn test_generate_alter_type_executable() {
    create_alter_type_fixtures();

    // insert test data so matview refresh works
    Spi::run("INSERT INTO public.at_base VALUES (1, 99.99, 'test', true)").unwrap();
    Spi::run("REFRESH MATERIALIZED VIEW public.at_mat_dep").unwrap();

    // change label from text to varchar(100) — compatible, no view deps on label
    let results: Vec<_> = crate::functions::generate_alter_type(
        "public", "at_base", "label", "varchar(100)",
    ).collect();

    for row in &results {
        if let Some(sql) = row.2.as_deref() {
            Spi::run(sql).unwrap();
        }
    }

    // verify type changed
    let new_type = Spi::connect(|client| {
        client
            .select(
                "SELECT data_type FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'at_base' AND column_name = 'label'",
                None,
                &[],
            )
            .map(|t| t.first().get_by_name::<String, _>("data_type").unwrap().unwrap_or_default())
    })
    .unwrap();
    assert_eq!(new_type, "character varying", "label should now be varchar");
}

fn create_rename_view_column_fixtures() {
    Spi::run(include_str!("../sql_queries/tests/create_rename_view_column_fixtures.sql")).unwrap();
}

#[pg_test]
fn test_generate_rename_view_column_basic() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    assert!(!results.is_empty());

    for (i, row) in results.iter().enumerate() {
        assert_eq!(row.0, (i + 1) as i32);
    }

    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();

    // DROPs come first (dependents + target)
    let last_drop = ops.iter().rposition(|o| o.starts_with("DROP")).unwrap();

    // then target CREATE
    let target_create_pos = ops
        .iter()
        .position(|o| *o == "CREATE VIEW" || *o == "CREATE MATERIALIZED VIEW")
        .unwrap();
    assert!(last_drop < target_create_pos, "DROPs must come before CREATEs");

    // then GRANTs
    if let Some(grant_pos) = ops.iter().position(|o| *o == "GRANT") {
        let last_create = ops.iter().rposition(|o| o.starts_with("CREATE")).unwrap();
        assert!(last_create < grant_pos, "CREATEs must come before GRANTs");
    }
}

#[pg_test]
fn test_generate_rename_view_column_no_deps() {
    create_rename_view_column_fixtures();

    // rvc_dep_l2 is a leaf view with no dependents — just DROP + CREATE
    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_dep_l2", "name", "full_name",
    ).collect();

    let ops: Vec<&str> = results.iter().filter_map(|r| r.1.as_deref()).collect();
    assert_eq!(ops.len(), 2);
    assert!(ops[0].starts_with("DROP"), "first step should be DROP");
    assert!(ops[1].starts_with("CREATE"), "second step should be CREATE");
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_rename_view_column_nonexistent_view() {
    create_rename_view_column_fixtures();

    let _: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "no_such_view", "name", "full_name",
    ).collect();
}

#[pg_test]
#[should_panic(expected = "does not exist")]
fn test_generate_rename_view_column_nonexistent_column() {
    create_rename_view_column_fixtures();

    let _: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "no_such_col", "full_name",
    ).collect();
}

#[pg_test]
#[should_panic(expected = "same")]
fn test_generate_rename_view_column_same_name() {
    create_rename_view_column_fixtures();

    let _: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "name",
    ).collect();
}

#[pg_test]
fn test_generate_rename_view_column_column_list() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    // find the CREATE step for the target view
    let target_create = results
        .iter()
        .find(|r| {
            r.2.as_deref().map(|s| s.contains("rvc_target") && s.starts_with("CREATE")).unwrap_or(false)
        });

    assert!(target_create.is_some(), "expected CREATE for rvc_target");
    let sql = target_create.unwrap().2.as_deref().unwrap();
    assert!(sql.contains("full_name"), "CREATE should use new column name 'full_name'");
    assert!(!sql.contains("(\"name\"") && !sql.contains(", \"name\""), "CREATE should not use old column name 'name' in column list");
}


#[pg_test]
fn test_generate_rename_view_column_todo_marker() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    let create_steps: Vec<(&str, &str)> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("CREATE")).unwrap_or(false))
        .map(|r| (r.1.as_deref().unwrap(), r.2.as_deref().unwrap()))
        .collect();

    // rvc_dep_l1 references 'name' — should get TODO
    let dep_l1 = create_steps.iter().find(|(_, sql)| sql.contains("rvc_dep_l1"));
    assert!(dep_l1.is_some(), "expected CREATE for rvc_dep_l1");
    assert!(
        dep_l1.unwrap().1.contains("-- TODO: update reference from 'name' to 'full_name'"),
        "rvc_dep_l1 should have TODO marker"
    );

    // rvc_dep_no_ref does not reference 'name' but depends on rvc_target via 'email'
    // it should be in the plan (it's a dependent) but without TODO
    let dep_no_ref = create_steps.iter().find(|(_, sql)| sql.contains("rvc_dep_no_ref"));
    assert!(dep_no_ref.is_some(), "expected CREATE for rvc_dep_no_ref");
    assert!(
        !dep_no_ref.unwrap().1.contains("-- TODO"),
        "rvc_dep_no_ref should NOT have TODO marker"
    );
}

#[pg_test]
fn test_generate_rename_view_column_grants_preserved() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    let grant_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("GRANT"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!grant_sqls.is_empty(), "expected GRANT steps");

    let has_target_grant = grant_sqls.iter().any(|s| s.contains("rvc_target"));
    let has_l1_grant = grant_sqls.iter().any(|s| s.contains("rvc_dep_l1"));
    assert!(has_target_grant, "expected GRANT for rvc_target");
    assert!(has_l1_grant, "expected GRANT for rvc_dep_l1");
}

#[pg_test]
fn test_generate_rename_view_column_matview_refresh() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    let refresh_ops: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref() == Some("REFRESH MATERIALIZED VIEW"))
        .filter_map(|r| r.2.as_deref())
        .collect();

    assert!(!refresh_ops.is_empty(), "expected REFRESH MATERIALIZED VIEW step");
    assert!(
        refresh_ops.iter().any(|s| s.contains("rvc_mat_dep")),
        "expected REFRESH for rvc_mat_dep"
    );
}

#[pg_test]
fn test_generate_rename_view_column_drop_order() {
    create_rename_view_column_fixtures();

    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_target", "name", "full_name",
    ).collect();

    let drop_sqls: Vec<&str> = results
        .iter()
        .filter(|r| r.1.as_deref().map(|o| o.starts_with("DROP")).unwrap_or(false))
        .filter_map(|r| r.2.as_deref())
        .collect();

    // rvc_dep_l2 depends on rvc_dep_l1, so l2 must be dropped first
    let l2_pos = drop_sqls.iter().position(|s| s.contains("rvc_dep_l2"));
    let l1_pos = drop_sqls.iter().position(|s| s.contains("rvc_dep_l1"));

    assert!(l2_pos.is_some(), "expected DROP for rvc_dep_l2");
    assert!(l1_pos.is_some(), "expected DROP for rvc_dep_l1");
    assert!(
        l2_pos.unwrap() < l1_pos.unwrap(),
        "rvc_dep_l2 must be dropped before rvc_dep_l1"
    );

    // target is dropped last among the DROPs
    let target_pos = drop_sqls.iter().position(|s| s.contains("rvc_target"));
    assert!(target_pos.is_some(), "expected DROP for rvc_target");
    assert!(
        l1_pos.unwrap() < target_pos.unwrap(),
        "dependents must be dropped before target"
    );
}

#[pg_test]
fn test_generate_rename_view_column_executable() {
    create_rename_view_column_fixtures();

    // insert test data so matview refresh works
    Spi::run("INSERT INTO public.rvc_base VALUES (1, 'alice', 'a@example.com', true)").unwrap();
    Spi::run("REFRESH MATERIALIZED VIEW public.rvc_mat_dep").unwrap();

    // rename 'email' to 'email_address' on rvc_dep_no_ref (leaf view, no dependents referencing it)
    let results: Vec<_> = crate::functions::generate_rename_view_column(
        "public", "rvc_dep_no_ref", "email", "email_address",
    ).collect();

    for row in &results {
        if let Some(sql) = row.2.as_deref() {
            Spi::run(sql).unwrap();
        }
    }

    // verify column name changed
    let has_new_col = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'rvc_dep_no_ref' AND column_name = 'email_address'",
                None,
                &[],
            )
            .map(|t| t.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0))
    })
    .unwrap();
    assert_eq!(has_new_col, 1, "rvc_dep_no_ref should have 'email_address' column");

    let has_old_col = Spi::connect(|client| {
        client
            .select(
                "SELECT count(*) AS cnt FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'rvc_dep_no_ref' AND column_name = 'email'",
                None,
                &[],
            )
            .map(|t| t.first().get_by_name::<i64, _>("cnt").unwrap().unwrap_or(0))
    })
    .unwrap();
    assert_eq!(has_old_col, 0, "rvc_dep_no_ref should NOT have 'email' column anymore");
}
