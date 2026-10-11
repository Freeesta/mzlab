use super::*;

#[test]
fn ammonium_step_has_two_readings() {
    let t = Tables::new(1);
    let r = t.readings(300.0, 317.02655, 5.0, 0.002);
    let names: Vec<&str> = r.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, vec![NH4_NAME, "NH3"]);
    assert!(r[0].ppm.abs() < 0.5);
}

#[test]
fn formate_in_negative_mode_and_loss() {
    let t = Tables::new(-1);
    let r = t.readings(200.0, 246.00548, 5.0, 0.002);
    assert!(r.iter().any(|x| x.name == "[M+FA-H]-") && r.iter().any(|x| x.name == "HCOOH"));
}

#[test]
fn dimer_is_found_and_inverted() {
    let t = Tables::new(1);
    let y = 2.0 * (250.1234 - t.proton) + t.proton;
    assert!(t.is_dimer(250.1234, y, 5.0, 0.002));
    assert!((t.dimer_base(y) - 250.1234).abs() < 1e-9);
}
