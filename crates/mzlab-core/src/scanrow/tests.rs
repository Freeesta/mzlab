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
