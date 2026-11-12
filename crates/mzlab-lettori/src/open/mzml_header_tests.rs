//! The header of an mzML with two instrument configurations: the model, the serial number and the components come from the first one only.
#![allow(clippy::unwrap_used)]
use super::*;

const HEAD: &str = r#"<mzML><instrumentConfigurationList count="2">
<instrumentConfiguration id="IC1"><cvParam cvRef="MS" accession="MS:1000031" name="instrument model" value=""/>
<cvParam cvRef="MS" accession="MS:1000529" name="instrument serial number" value="S-1"/>
<componentList count="1"><source order="1"><cvParam cvRef="MS" accession="MS:1000073" name="electrospray ionization" value=""/></source></componentList></instrumentConfiguration>
<instrumentConfiguration id="IC2"><cvParam cvRef="MS" accession="MS:1000121" name="other model" value=""/>
<cvParam cvRef="MS" accession="MS:1000529" name="instrument serial number" value="S-2"/>
<componentList count="1"><analyzer order="1"><cvParam cvRef="MS" accession="MS:1000081" name="quadrupole" value=""/></analyzer></componentList></instrumentConfiguration>
</instrumentConfigurationList><run id="r1" startTimeStamp="2020-01-02T03:04:05Z">"#;

#[test]
fn only_the_first_instrument_configuration_makes_the_header() {
    let h = header_from(HEAD, "a.mzML", &Limits::sito()).unwrap();
    assert_eq!(
        h.instrument,
        Some(("MS:1000031".into(), "instrument model".into()))
    );
    assert_eq!(h.serial.as_deref(), Some("S-1"));
    assert_eq!(
        h.components,
        [(
            "source".to_string(),
            "MS:1000073".to_string(),
            "electrospray ionization".to_string()
        )]
    );
    assert_eq!(
        (h.run.as_str(), h.acquired.as_deref()),
        ("r1", Some("2020-01-02T03:04:05Z"))
    );
}
