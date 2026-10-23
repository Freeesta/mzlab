//! The retention time of a library entry: read from MSP, MGF and MassBank records, stored in the `.mzlib` and read back (minutes).
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{parse_all, Format};
use mzlab_lib::read::Library;

fn rts(fmt: Format, text: &str) -> Vec<Option<f64>> {
    parse_all(fmt, text.as_bytes())
        .0
        .iter()
        .map(|s| s.rt)
        .collect()
}

#[test]
fn msp_keys_and_units() {
    let msp = "Name: A\nPrecursorMZ: 300.1\nRT: 5.25\nNum Peaks: 1\n100 10\n\n\
Name: B\nPrecursorMZ: 301.1\nRetentionTime: 312 s\nNum Peaks: 1\n100 10\n\n\
Name: C\nPrecursorMZ: 302.1\nRetention_time: 4,5 min\nNum Peaks: 1\n100 10\n\n\
Name: D\nPrecursorMZ: 303.1\nNum Peaks: 1\n100 10\n\n\
Name: E\nPrecursorMZ: 304.1\nRT: n/a\nNum Peaks: 1\n100 10\n";
    let r = rts(Format::Msp, msp);
    assert_eq!(r.len(), 5);
    assert_eq!(r[0], Some(5.25));
    assert_eq!(r[1], Some(5.2));
    assert_eq!(r[2], Some(4.5));
    assert_eq!(r[3], None);
    assert_eq!(r[4], None);
}

#[test]
fn mgf_seconds_become_minutes() {
    let mgf = "BEGIN IONS\nTITLE=x\nPEPMASS=300.1\nRTINSECONDS=150\n100 10\n200 20\nEND IONS\n";
    assert_eq!(rts(Format::Mgf, mgf), [Some(2.5)]);
}

#[test]
fn massbank_chromatography_line() {
    let mb = "ACCESSION: MB000001\nRECORD_TITLE: t\nCH$NAME: n\nAC$CHROMATOGRAPHY: RETENTION_TIME 7.5 min\n\
MS$FOCUSED_ION: PRECURSOR_M/Z 300.1\nPK$PEAK: m/z int. rel.int.\n  100 10 999\n  200 20 500\n//\n";
    assert_eq!(rts(Format::MassBank, mb), [Some(7.5)]);
}

#[test]
fn the_retention_time_survives_the_mzlib() {
    let msp = "Name: A\nPrecursorMZ: 300.1\nRT: 5.25\nNum Peaks: 3\n100 10\n150 20\n200 30\n\n\
Name: B\nPrecursorMZ: 400.1\nNum Peaks: 3\n100 10\n150 20\n200 30\n";
    let (set, _) = parse_all(Format::Msp, msp.as_bytes());
    let (bytes, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(bytes, 1 << 20).unwrap();
    lib.verify().unwrap();
    let got: Vec<Option<f64>> = (0..lib.n_spectra())
        .map(|i| lib.rec(i).unwrap().rt_min())
        .collect();
    assert_eq!(got, [Some(5.25), None]);
}
