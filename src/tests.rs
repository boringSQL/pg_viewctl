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
