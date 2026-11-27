use super::*;

fn row(first: &str, second: &str) -> Value {
    let text = format!("[{first},{second},1,1.5,1,10.0,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null]");
    crate::json::parse(text.as_bytes()).unwrap()
}

#[test]
fn a_negative_offset_or_scan_number_is_no_scan() {
    assert!(scan_of_row(&row("5", "9"), false).is_some());
    assert!(scan_of_row(&row("-5", "9"), false).is_none());
    assert!(scan_of_row(&row("5", "-9"), false).is_none());
    assert!(scan_of_row(&row("null", "3"), true).is_some());
    assert!(scan_of_row(&row("null", "-3"), true).is_none());
}

#[test]
fn the_fill_time_is_column_23_and_a_row_without_it_has_none() {
    let with = |fill: &str| {
        let text = format!("[5,9,1,1.5,1,10.0,null,null,null,null,null,null,null,null,null,null,null,null,null,\"scan=1\",null,null,{fill}]");
        scan_of_row(&crate::json::parse(text.as_bytes()).unwrap(), false).unwrap()
    };
    // the shortest text of the `f32` reads back to the same `f32`
    assert_eq!(with("7.3").fill_ms(), Some(f64::from(7.3f32)));
    // a missing, null, zero or negative time states nothing
    assert_eq!(scan_of_row(&row("5", "9"), false).unwrap().fill_ms(), None);
    for bad in ["null", "0.0", "-2.0"] {
        assert_eq!(with(bad).fill_ms(), None, "{bad}");
    }
}
