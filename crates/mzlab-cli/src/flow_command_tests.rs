//! `mzlab-cli flow command`: the script it prints, run by a shell, writes the plan and calls `flow run` with the right arguments, and that run writes the
//! same files as a run of the plan itself.

use std::process::Command;

use crate::flow_run_tests::{fixture, flow_run, folder, p};

fn execute(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_copied_command_reruns_the_plan_with_the_same_bytes() {
    let (dir, _) = fixture("command");
    let (plan, data) = (dir.join("plan.json"), dir.join("data"));
    let o = execute(&["flow", "command", p(&plan)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let script = String::from_utf8(o.stdout).unwrap();
    let script = script.replace("/path/to/the/folder/with/your/files", p(&data));

    // a shell runs the script; `mzlab-cli` is a shim that records its arguments
    let (work, bin) = (dir.join("work"), dir.join("bin"));
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    let shim = bin.join("mzlab-cli");
    std::fs::write(&shim, "#!/bin/sh\nprintf '%s\\n' \"$@\" > args.txt\n").unwrap();
    Command::new("chmod")
        .args(["+x", p(&shim)])
        .status()
        .unwrap();
    let path = format!("{}:{}", p(&bin), std::env::var("PATH").unwrap());
    let st = Command::new("sh")
        .args(["-c", &script])
        .env("PATH", path)
        .current_dir(&work)
        .status()
        .unwrap();
    assert!(st.success());

    let args = std::fs::read_to_string(work.join("args.txt")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    let written = work.join("mzlab-run/plan.json");
    assert_eq!(
        args,
        [
            "flow",
            "run",
            "mzlab-run/plan.json",
            "--in",
            p(&data),
            "--out",
            "mzlab-run/out"
        ]
    );
    // the plan the shell wrote is the canonical text, which parses and checks like the original
    let canon = std::fs::read(&written).unwrap();
    let again = mzlab_core::flow::plan_json(&mzlab_core::flow::parse(&canon).unwrap());
    assert_eq!(String::from_utf8(canon).unwrap(), again);

    // running what the script asks for gives the same files as running the original plan
    let (a, b) = (dir.join("out_a"), work.join("mzlab-run/out"));
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&a)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = flow_run(&[p(&written), "--in", p(&data), "--out", p(&b)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(folder(&a), folder(&b));
}

#[test]
fn a_missing_plan_is_an_orderly_error() {
    let o = execute(&["flow", "command", "/no/such/plan.json"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("plan.json"));
    assert_eq!(execute(&["flow", "command"]).code, 2);
}
