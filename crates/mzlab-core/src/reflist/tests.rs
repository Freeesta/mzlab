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
