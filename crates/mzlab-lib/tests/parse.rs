//! The parser keeps the rules of the JS `LibParser` (tests/test_libreria.py) and adds JSON and MassBank records.
use mzlab_lib::parse::{detect, parse_all, Format, Parser, Stats};
use mzlab_lib::spec::Spectrum;

const MSP: &str = "Name: Alfa
PrecursorMZ: 300.1234
Precursor_type: [M+H]+
Ion_mode: Positive
Collision_energy: 30
Instrument_type: HCD
Formula: C10H10O
InChIKey: AAAAAAAAAAAAAA-BBBBBBBBBB-N
Num Peaks: 3
100.5\t1000
150.25\t500
200.1\t250

Name: Beta, negative
PRECURSORMZ: 255.5
Precursor_type: [M-H]-
Num Peaks: 2
80.0 10
120.5 20

Name: Senza precursore
Num Peaks: 2
50 10
60 20

Name: Senza picchi
PrecursorMZ: 400
Num Peaks: 0

Name: Rotto
PrecursorMZ: abc
Num Peaks: 1
10 10

Name: Gamma NIST style
PrecursorMZ: 410.2
Num Peaks: 3
50 100; 60 200; 70 300;
";

const MGF: &str = "BEGIN IONS
NAME=Delta
PEPMASS=500.25 12345
CHARGE=1+
IONMODE=Positive
100.0 10
200.0 20
END IONS
BEGIN IONS
NAME=Rotto
PEPMASS=600.0
100.0 10
BEGIN IONS
NAME=Epsilon
PEPMASS=700.5
CHARGE=1-
300.0 5
END IONS
BEGIN IONS
NAME=SenzaPicchi
PEPMASS=800.0
END IONS
";

fn chunked(fmt: Format, text: &str, chunk: usize) -> (Vec<Spectrum>, Stats) {
    let mut out = Vec::new();
    let mut p = Parser::new(fmt, |s| out.push(s));
    for c in text.as_bytes().chunks(chunk) {
        p.push(c);
    }
    p.end();
    let st = p.stats;
    (out, st)
}

#[test]
fn msp_blocks_fields_and_discarded() {
    let (out, st) = parse_all(Format::Msp, MSP.as_bytes());
    let names: Vec<_> = out.iter().map(|s| s.meta.name.as_str()).collect();
    assert_eq!(names, ["Alfa", "Beta, negative", "Gamma NIST style"]);
    let (a, b, g) = (&out[0], &out[1], &out[2]);
    assert_eq!(a.prec, 300.1234);
    assert_eq!(a.pol, 1);
    assert_eq!(a.meta.adduct, "[M+H]+");
    assert_eq!(a.meta.ce, "30");
    assert_eq!(a.meta.instrument, "HCD");
    assert_eq!(a.meta.formula, "C10H10O");
    assert_eq!(a.meta.inchikey, "AAAAAAAAAAAAAA-BBBBBBBBBB-N");
    assert_eq!(a.mz, [100.5, 150.25, 200.1]);
    assert_eq!(a.it, [1000.0, 500.0, 250.0]);
    assert_eq!(b.pol, -1);
    assert_eq!(g.pol, 0);
    assert_eq!(g.mz.len(), 3);
    assert_eq!(
        st,
        Stats {
            read: 3,
            dropped: 3,
            no_prec: 2,
            no_peaks: 1,
            broken: 0
        }
    );
}

#[test]
fn msp_is_the_same_in_any_chunking() {
    let whole = parse_all(Format::Msp, MSP.as_bytes());
    for chunk in [1, 7, 50, 100_000] {
        assert_eq!(chunked(Format::Msp, MSP, chunk), whole, "chunk {chunk}");
    }
}

#[test]
fn mgf_with_a_broken_block() {
    let (out, st) = parse_all(Format::Mgf, MGF.as_bytes());
    let names: Vec<_> = out.iter().map(|s| s.meta.name.as_str()).collect();
    assert_eq!(names, ["Delta", "Epsilon"]);
    assert_eq!((out[0].prec, out[0].pol, out[1].pol), (500.25, 1, -1));
    assert_eq!((st.read, st.broken, st.no_peaks, st.dropped), (2, 1, 1, 2));
    assert_eq!(chunked(Format::Mgf, MGF, 3), (out, st));
}

#[test]
fn utf8_split_across_chunks_and_crlf() {
    let t = "Name: Caffè «x»\r\nPrecursorMZ: 195.0877\r\nNum Peaks: 1\r\n138.06 100\r\n";
    let (out, _) = chunked(Format::Msp, t, 1);
    assert_eq!(out[0].meta.name, "Caffè «x»");
    assert_eq!(out[0].mz, [138.06]);
}

#[test]
fn format_detection() {
    let f = |n: &str, h: &str| detect(n, h.as_bytes()).map_err(|e| e.key);
    assert_eq!(f("a.MSP", ""), Ok(Format::Msp));
    assert_eq!(f("b.mgf", ""), Ok(Format::Mgf));
    assert_eq!(f("c.lib", ""), Err("lib.err.unsupported"));
    assert_eq!(f("d.mzVault", ""), Err("lib.err.unsupported"));
    assert_eq!(f("x.txt", "BEGIN IONS\n"), Ok(Format::Mgf));
    assert_eq!(f("x", "Name: a\n"), Ok(Format::Msp));
    assert_eq!(f("x", "zzz"), Err("lib.err.unknown"));
    assert_eq!(f("x.json", ""), Ok(Format::Json));
    assert_eq!(f("x", "  [ {\"a\":1}"), Ok(Format::Json));
    assert_eq!(
        f("MSBNK-X-1.txt", "ACCESSION: MSBNK-X-1\nRECORD_TITLE: a\n"),
        Ok(Format::MassBank)
    );
}

/// A MassBank record written by hand (the fields the parser reads, in the order of the format).
const MASSBANK: &str = "ACCESSION: MSBNK-Test-TS000001
RECORD_TITLE: Compound alpha; LC-ESI-QFT; MS2; CE: 30; R=17500; [M+H]+
DATE: 2024.01.01
AUTHORS: A. Author, B. Author
LICENSE: CC BY
COMMENT: synthetic record
CH$NAME: Compound alpha
CH$NAME: Alternative name
CH$COMPOUND_CLASS: N/A
CH$FORMULA: C9H8O4
CH$EXACT_MASS: 180.04226
CH$SMILES: CC(=O)OC1=CC=CC=C1C(=O)O
CH$IUPAC: InChI=1S/C9H8O4/c1-6(10)13-8-5-3-2-4-7(8)9(11)12/h2-5H,1H3,(H,11,12)
CH$LINK: INCHIKEY BSYNRYMUTXBXSQ-UHFFFAOYSA-N
AC$INSTRUMENT: Q Exactive
AC$INSTRUMENT_TYPE: LC-ESI-QFT
AC$MASS_SPECTROMETRY: MS_TYPE MS2
AC$MASS_SPECTROMETRY: ION_MODE POSITIVE
AC$MASS_SPECTROMETRY: FRAGMENTATION_MODE HCD
AC$MASS_SPECTROMETRY: COLLISION_ENERGY 30 (NCE)
MS$FOCUSED_ION: PRECURSOR_M/Z 181.0495
MS$FOCUSED_ION: PRECURSOR_TYPE [M+H]+
PK$SPLASH: splash10-0000-0000000000-0000000000
PK$NUM_PEAK: 3
PK$PEAK: m/z int. rel.int.
  121.0284 12345.6 999
  139.0390 500.5 40
  163.0390 1000 80
//
ACCESSION: MSBNK-Test-TS000002
CH$NAME: Compound beta
LICENSE: CC BY-SA
MS$FOCUSED_ION: PRECURSOR_M/Z 100.0
AC$MASS_SPECTROMETRY: ION_MODE NEGATIVE
PK$PEAK: m/z int. rel.int.
  50.0 10 100
//
";

#[test]
fn massbank_records() {
    let (out, st) = parse_all(Format::MassBank, MASSBANK.as_bytes());
    assert_eq!(st.read, 2, "{st:?}");
    let a = &out[0];
    assert_eq!(
        (
            a.meta.accession.as_str(),
            a.meta.name.as_str(),
            a.meta.formula.as_str()
        ),
        ("MSBNK-Test-TS000001", "Compound alpha", "C9H8O4")
    );
    assert_eq!(a.meta.smiles, "CC(=O)OC1=CC=CC=C1C(=O)O");
    assert_eq!(a.meta.inchikey, "BSYNRYMUTXBXSQ-UHFFFAOYSA-N");
    assert_eq!(
        (
            a.meta.instrument.as_str(),
            a.meta.adduct.as_str(),
            a.meta.ce.as_str()
        ),
        ("LC-ESI-QFT", "[M+H]+", "30 (NCE)")
    );
    assert_eq!(
        (a.meta.license.as_str(), a.meta.authors.as_str()),
        ("CC BY", "A. Author, B. Author")
    );
    assert_eq!((a.prec, a.pol), (181.0495, 1));
    assert_eq!(a.mz, [121.0284, 139.0390, 163.0390]);
    assert_eq!(
        a.it,
        [12345.6, 500.5, 1000.0],
        "the second column is the absolute intensity"
    );
    let b = &out[1];
    assert_eq!(
        (b.meta.accession.as_str(), b.pol, b.meta.license.as_str()),
        ("MSBNK-Test-TS000002", -1, "CC BY-SA")
    );
    assert_eq!(chunked(Format::MassBank, MASSBANK, 5), (out, st));
}

#[test]
fn json_lines_of_mzmine() {
    let t = r#"{"compound_name":"Alpha","precursor_mz":300.5,"adduct":"[M+H]+","polarity":"POSITIVE","collision_energy":[30.0],"formula":"C1H2","smiles":"CC","inchi_key":"AAAAAAAAAAAAAA-BBBBBBBBBB-N","instrument_type":"Orbitrap","peaks":[[100.0,10.0],[200.5,20.0]]}
{"compound_name":"Beta","precursor_mz":"250.25","adduct":"[M-H]-","polarity":"NEGATIVE","peaks":"[[50.0,1.0],[60.0,2.0],[70.0,3.0]]"}
{"compound_name":"No peaks","precursor_mz":100.0,"peaks":[]}
{"compound_name": broken
"#;
    let (out, st) = parse_all(Format::Json, t.as_bytes());
    assert_eq!(out.len(), 2, "{st:?}");
    assert_eq!(
        (
            out[0].meta.name.as_str(),
            out[0].prec,
            out[0].pol,
            out[0].meta.ce.as_str()
        ),
        ("Alpha", 300.5, 1, "30.0")
    );
    assert_eq!(
        (
            out[0].meta.formula.as_str(),
            out[0].meta.smiles.as_str(),
            out[0].meta.instrument.as_str()
        ),
        ("C1H2", "CC", "Orbitrap")
    );
    assert_eq!(
        out[1].mz,
        [50.0, 60.0, 70.0],
        "peaks given as a JSON string are decoded"
    );
    assert_eq!((out[1].prec, out[1].pol), (250.25, -1));
    assert_eq!(st.no_peaks, 1);
    assert!(st.broken >= 1);
    assert_eq!(chunked(Format::Json, t, 3), (out, st));
}

#[test]
fn json_array_of_mona_and_gnps() {
    let t = r#"[
 {"id":"MoNA-1","compound":[{"names":[{"name":"Alpha"},{"name":"Other"}],"inchiKey":"AAAAAAAAAAAAAA-BBBBBBBBBB-N","metaData":[{"name":"molecular formula","value":"C2H6O"}]}],
  "metaData":[{"name":"precursor m/z","value":181.05},{"name":"precursor type","value":"[M+H]+"},{"name":"ionization mode","value":"positive"},{"name":"collision energy","value":"20 eV"},{"name":"instrument type","value":"LC-ESI-QTOF"}],
  "spectrum":"100.0:50 150.5:100 {braces} 200:25","library":{"library":"X"}},
 {"Compound_Name":"Gamma","Precursor_MZ":"400.1","Ion_Mode":"Negative","Adduct":"M-H","Instrument":"Q-TOF","Smiles":"C","INCHI":"InChI=1S/x","peaks_json":"[[10.0,1.0],[20.0,5.0]]"}
]"#;
    let (out, st) = parse_all(Format::Json, t.as_bytes());
    assert_eq!(st.read, 2, "{st:?}");
    let a = &out[0];
    assert_eq!(
        (
            a.meta.name.as_str(),
            a.meta.inchikey.as_str(),
            a.meta.formula.as_str()
        ),
        ("Alpha", "AAAAAAAAAAAAAA-BBBBBBBBBB-N", "C2H6O")
    );
    assert_eq!(
        (
            a.prec,
            a.pol,
            a.meta.adduct.as_str(),
            a.meta.ce.as_str(),
            a.meta.instrument.as_str()
        ),
        (181.05, 1, "[M+H]+", "20 eV", "LC-ESI-QTOF")
    );
    assert_eq!(a.mz, [100.0, 150.5, 200.0]);
    assert_eq!(a.it, [50.0, 100.0, 25.0]);
    assert_eq!(
        (
            out[1].meta.name.as_str(),
            out[1].pol,
            out[1].prec,
            out[1].mz.len()
        ),
        ("Gamma", -1, 400.1, 2)
    );
    assert_eq!(chunked(Format::Json, t, 1), (out, st));
}

#[test]
fn very_long_names_and_huge_peak_lists_do_not_panic() {
    let long = "N".repeat(100_000);
    let t = format!("Name: {long}\nPrecursorMZ: 100\nNum Peaks: 1\n50 10\n");
    let (out, _) = parse_all(Format::Msp, t.as_bytes());
    assert_eq!(out[0].meta.name.len(), 2048);
    let mut big = String::from("Name: Many\nPrecursorMZ: 100\n");
    for i in 0..250_000 {
        big.push_str(&format!("{} 10\n", 10 + i));
    }
    let (out, _) = parse_all(Format::Msp, big.as_bytes());
    assert_eq!(out[0].mz.len(), 200_000);
}

#[test]
fn garbage_never_panics() {
    let mut seed = 12345u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as u8
    };
    let pieces: &[&[u8]] = &[
        b"Name:",
        b"BEGIN IONS",
        b"END IONS",
        b"PEPMASS=",
        b"{",
        b"}",
        b"[",
        b"]",
        b"\"",
        b"\\",
        b"\n",
        b"//",
        b"ACCESSION:",
        b"PK$PEAK:",
        b"12.5 ",
        b"\xff\xfe",
        b"Num Peaks: 2\n",
    ];
    for fmt in [Format::Msp, Format::Mgf, Format::MassBank, Format::Json] {
        let mut data = Vec::new();
        for _ in 0..20_000 {
            data.extend_from_slice(pieces[rnd() as usize % pieces.len()]);
        }
        let mut n = 0;
        let mut p = Parser::new(fmt, |_| n += 1);
        p.push(&data);
        p.end();
    }
}
