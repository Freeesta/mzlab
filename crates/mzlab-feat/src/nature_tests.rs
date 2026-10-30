//! The nature rule of a series (`nature.rs`): polarity and unit resolution in HR-, LR+ and mixed series, and what the plan writes.

use super::*;

#[test]
fn negative_hr_positive_lr_and_mixed_series() {
    let neg_hr = of_facts([(-1, true), (-1, true)]);
    assert_eq!(
        neg_hr,
        Nature {
            polarity: Some(-1),
            unit: false
        }
    );
    let pos_lr = of_facts([(1, false), (1, false), (1, false)]);
    assert_eq!(
        pos_lr,
        Nature {
            polarity: Some(1),
            unit: true
        }
    );
    // an even mix of polarities decides none; one high-resolution file makes the series not unit
    let mixed = of_facts([(1, false), (-1, true)]);
    assert_eq!(
        mixed,
        Nature {
            polarity: None,
            unit: false
        }
    );
    // the majority decides, a file of unknown polarity counts for none
    assert_eq!(
        of_facts([(-1, false), (-1, false), (1, false), (0, false)]).polarity,
        Some(-1)
    );
    assert_eq!(
        of_facts([]),
        Nature {
            polarity: None,
            unit: false
        }
    );
}

#[test]
fn the_plan_wins_and_the_detection_closes_the_polarity() {
    let n = Nature {
        polarity: Some(-1),
        unit: true,
    };
    let names = |v: Vec<(String, Value, Value)>| v.into_iter().map(|e| e.0).collect::<Vec<_>>();
    assert_eq!(names(estimates(&|_| false, &n)), ["polarity", "nominal"]);
    assert_eq!(names(estimates(&|k| k == "polarity", &n)), ["nominal"]);
    assert!(estimates(&|_| true, &n).is_empty());
    assert_eq!(open_polarity(None, &n), Some(-1));
    assert_eq!(open_polarity(Some(0), &n), Some(-1));
    assert_eq!(open_polarity(Some(1), &n), None);
    assert_eq!(
        estimates_json(&estimates(&|_| false, &n)),
        r#"[{"name":"polarity","value":-1,"proof":"scan polarity of the files"},{"name":"nominal","value":true,"proof":"unit resolution of the files"}]"#
    );
}
