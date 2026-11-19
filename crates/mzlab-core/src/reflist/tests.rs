use super::*;

fn run(csv: &str) -> Value {
    let body = write(&Obj::new().put("csv", text(csv)).done());
    parse(route(&body).unwrap().as_bytes()).unwrap()
}

fn n(v: &Value, k: &str) -> usize {
    v.get(k).unwrap().arr().len()
}

#[test]
fn rows_with_mz_or_formula_become_ions() {
    let v = run("mz;name;formula;polarity\n100.5;A;;+\n;B;C6H12O6;\n;C;C6H12O6;-\n");
    assert_eq!((n(&v, "items"), n(&v, "errors")), (3, 0));
    assert_eq!(v.get("compounds").and_then(Value::f64), Some(3.0)); // distinct names: A, B, C
    let it = v.get("items").unwrap().arr();
    assert_eq!(it[0].get("pol").and_then(Value::f64), Some(1.0));
    assert_eq!(it[1].get("adduct").and_then(Value::str), Some("[M+H]+"));
    assert_eq!(it[2].get("adduct").and_then(Value::str), Some("[M-H]-"));
    assert_eq!(it[1].get("mz").and_then(Value::f64), Some(181.07066));
}

#[test]
fn wrong_rows_carry_their_file_line() {
    let v = run("# note\nmz,name,formula,polarity,adduct\n100,A,,,\nx,B,,,\n,C,,,\n,D,Zz9,,\n5,E,,up,\n5,F,,,[M+X]+\n");
    let e = v.get("errors").unwrap().arr();
    let got: Vec<(f64, &str)> = e
        .iter()
        .map(|r| {
            (
                r.get("line").and_then(Value::f64).unwrap(),
                r.get("key").and_then(Value::str).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (4.0, "err.reflist.mz"),
            (5.0, "err.reflist.noMass"),
            (6.0, "err.reflist.formula"),
            (7.0, "err.reflist.polarity"),
            (8.0, "err.reflist.adduct")
        ]
    );
    assert_eq!(n(&v, "items"), 1);
}

#[test]
fn header_and_request_problems() {
    assert_eq!(n(&run("a;b\n1;2\n"), "errors"), 1);
    assert_eq!(n(&run(""), "errors"), 1);
    assert!(route("{}").is_err());
}

#[test]
fn norman_style_headers_and_decimal_comma_are_read() {
    let v = run("Compound_Name;Molecular_Formula;Monoisotopic_Mass;SMILES;Ionization_Mode\nCaffeine;C8H10N4O2;194,0804;CN1C=NC2=C1C(=O)N(C(=O)N2C)C;positive\n");
    assert_eq!((n(&v, "items"), n(&v, "errors")), (1, 0));
    let it = &v.get("items").unwrap().arr()[0];
    assert_eq!(it.get("name").and_then(Value::str), Some("Caffeine"));
    assert_eq!(it.get("mz").and_then(Value::f64), Some(194.0804));
    assert_eq!(it.get("pol").and_then(Value::f64), Some(1.0));
}

#[test]
fn a_header_with_monoisotopicmass_and_a_quoted_decimal_comma_gives_the_number() {
    let v = run("name,monoisotopicmass\nA,\"150,5\"\n");
    let it = &v.get("items").unwrap().arr()[0];
    assert_eq!(it.get("mz").and_then(Value::f64), Some(150.5));
}

fn look(extra: &str, mz: &[f64], pol: &[i64], group: &[i64]) -> Vec<(usize, f64)> {
    let j = |a: &[String]| a.join(",");
    let body = format!(
        "{{{extra},\"items\":{{\"mz\":[{}],\"pol\":[{}],\"group\":[{}]}}}}",
        j(&mz.iter().map(|v| v.to_string()).collect::<Vec<_>>()),
        j(&pol.iter().map(|v| v.to_string()).collect::<Vec<_>>()),
        j(&group.iter().map(|v| v.to_string()).collect::<Vec<_>>())
    );
    let v = parse(lookup::route(&body).unwrap().as_bytes()).unwrap();
    v.get("hits")
        .unwrap()
        .arr()
        .iter()
        .map(|h| {
            (
                h.get("i").and_then(Value::f64).unwrap() as usize,
                h.get("diff").and_then(Value::f64).unwrap(),
            )
        })
        .collect()
}

#[test]
fn ppm_window_orders_by_error_and_reports_a_both_polarity_ion_once() {
    // 300 ppm... 5 ppm at 500 is 2.5 mDa: 500.001 (2nd), 500.0005 (1st), 500.004 out of the window
    let h = look(
        "\"mz\":500,\"unit\":\"ppm\",\"tol\":5",
        &[500.001, 500.0005, 500.004],
        &[0, 0, 0],
        &[0, 1, 2],
    );
    assert_eq!(h.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1, 0]);
}

#[test]
fn da_window_and_polarity_filter() {
    let mz = [100.2, 99.9, 100.4, 100.7];
    let all = look(
        "\"mz\":100,\"unit\":\"da\",\"tol\":0.5",
        &mz,
        &[1, -1, 1, 1],
        &[0, 1, 2, 3],
    );
    assert_eq!(all.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1, 0, 2]);
    let neg = look(
        "\"mz\":100,\"pol\":\"-\",\"unit\":\"da\"",
        &mz,
        &[1, -1, 1, 1],
        &[0, 1, 2, 3],
    );
    assert_eq!(neg.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1]);
}

#[test]
fn same_group_is_one_hit_and_a_low_mass_keeps_one_milli_dalton() {
    let h = look(
        "\"mz\":50,\"unit\":\"ppm\",\"tol\":5",
        &[50.0009, 50.0009],
        &[1, 1],
        &[7, 7],
    );
    assert_eq!(h.len(), 1); // 5 ppm of 50 is 0.25 mDa, the floor is 1 mDa
}

#[test]
fn a_broken_request_gives_the_request_error() {
    for b in [
        "{}",
        "{\"mz\":1,\"items\":{\"mz\":[1],\"pol\":[],\"group\":[]}}",
        "{\"mz\":1,\"unit\":\"x\",\"items\":{}}",
    ] {
        assert_eq!(
            lookup::route(b).unwrap_err().key,
            "err.reflist.request",
            "{b}"
        );
    }
}
