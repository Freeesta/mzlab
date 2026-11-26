//! `commands --find` for the one-call routes of many masses (`trace xic --mz LIST` / `--sweep`, `raw xic --mz LIST`, `features.union`): the words of
//! the task, not the flag names, reach them and the one-mass command names them as its `bulk`.

use super::{search, words};

fn top(q: &str, n: usize) -> Vec<String> {
    search(&words(q))
        .into_iter()
        .take(n)
        .map(|(it, _, _)| it.name)
        .collect()
}

/// A user who only knows the word of the task (a sweep over every mass, many masses, a list of ions) reaches the one-call routes.
#[test]
fn sweep_and_many_masses_find_the_one_call_xic_routes() {
    for (q, want) in [
        ("sweep", "trace xic"),
        ("xic every mass from 100 to 500 step 0.5", "trace xic"),
        ("many masses", "trace xic"),
        ("tens of thousands of calls", "trace xic"),
        ("all targets", "features.union"),
    ] {
        let got = top(q, 5);
        assert!(
            got.iter().any(|g| g == want),
            "{q:?}: {want} not in {got:?}"
        );
    }
    let find = |w: &str| {
        let a: Vec<String> = ["commands", "--find", w, "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let o = crate::execute(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        serde_json::from_slice::<serde_json::Value>(&o.stdout).unwrap()
    };
    let bulk = |v: &serde_json::Value, name: &str| {
        v["matches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["name"] == name)
            .and_then(|x| x["bulk"].as_str().map(String::from))
            .unwrap_or_default()
    };
    let v = find("raw xic many masses");
    assert!(
        bulk(&v, "raw xic").contains("raw xic FILE... --mz M1,M2,..."),
        "{v}"
    );
    assert!(bulk(&v, "trace xic").contains("--sweep"), "{v}");
}
