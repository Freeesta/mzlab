//! The desktop run path (`mzlab_cli::flow::run_plan`, the call `flow_run.rs` makes) refuses a plan with no step (`err.flow.empty`), as `flow run` does, so a
//! misspelt key never reads as a successful run.

#[test]
fn an_empty_plan_is_refused_on_the_run_path() {
    let dir = std::env::temp_dir().join(format!("mzlab-flow-empty-{}", std::process::id()));
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[],"steps":[]}"#;
    let e = mzlab_cli::flow::run_plan(plan.as_bytes(), &dir, &work, 1, None)
        .expect_err("an empty plan must not run");
    assert_eq!(e.key, "err.flow.check");
    assert!(e.to_string().contains("err.flow.empty"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
