use super::*;

fn e(time: f32, message: &str) -> ErrorEntry {
    ErrorEntry {
        time,
        message: message.into(),
    }
}

#[test]
fn an_entry_keeps_its_time_and_text_and_the_level_only_when_the_text_says_it() {
    let v = voci(&[
        e(1.5, "Warning: spray unstable"),
        e(12.0, "Info - reagent vial 1 configuration:"),
        e(20.25, "Calibration failed"),
        e(30.0, "Informed consent"),
    ]);
    let j: Vec<String> = v.iter().map(V::json).collect();
    assert_eq!(
        j,
        [
            r#"{"t_min":1.5,"testo":"Warning: spray unstable","livello":"Warning"}"#,
            r#"{"t_min":12,"testo":"Info - reagent vial 1 configuration:","livello":"Info"}"#,
            r#"{"t_min":20.25,"testo":"Calibration failed"}"#,
            r#"{"t_min":30,"testo":"Informed consent"}"#,
        ]
    );
}
