//! The synthetic SCIEX `.wiff` of the CI (written by `tools/cfb_scrivi.py`, invented numbers): DAD matrix and its header check, MRM parameters,
//! LC gradients, device channels, DDE precursors in cycle order, TOF calibration, `Itc` and the TDC block decoder. Needs python3.

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

use mzlab_core::methodinfo::{method_of_sample, Kind};
use mzlab_core::ole::Ole;
use mzlab_core::wiff::dad::{
    dde_records, decode_tof_block, device_channels, DadData, TofSampleData,
};
use mzlab_core::wiff::{container, Wiff};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write_synthetic_ci_wiff(out: &Path) {
    let script = concat!(
        "import struct, sys\n",
        "from pathlib import Path\n",
        "sys.path.insert(0, str(Path(sys.argv[1]) / 'tools'))\n",
        "import cfb_scrivi\n",
        "out = Path(sys.argv[2])\n",
        "dad_hdr = bytearray(86); struct.pack_into('<I', dad_hdr, 4, 4); struct.pack_into('<II', dad_hdr, 34, 3, 5); struct.pack_into('<ffff', dad_hdr, 46, 0.012, 200.0, 208.0, 2.0)\n",
        "rows = [[1.0, 2.0, 3.0, 4.0, 5.0], [0.5, 1.5, 2.5, 3.5, 4.5], [2.0, 4.0, 6.0, 8.0, 10.0]]\n",
        "dad_idx = bytearray(32) + b''.join(struct.pack('<ff', (i+1)*0.004, sum(r)) for i, r in enumerate(rows))\n",
        "dad_rtd = bytearray(32) + b''.join(struct.pack('<5f', *r) for r in rows)\n",
        "exp_hdr = bytearray(180); struct.pack_into('<I', exp_hdr, 4, 5); struct.pack_into('<I', exp_hdr, 122, 4); struct.pack_into('<I', exp_hdr, 176, 2)\n",
        "par = lambda n, v: struct.pack('<H', len(n)*2) + n.encode('utf-16le') + struct.pack('<fff', v, v, 0.0)\n",
        "ent = lambda q1, q3, dw, lb, dp, ep, cep, ce, cxp: struct.pack('<ffffIH', q1, 0.0, q3, dw, 0, len(lb)*2) + lb.encode('utf-16le') + par('DP', dp) + par('EP', ep) + par('CEP', cep) + par('CE', ce) + par('CXP', cxp)\n",
        "mrex = bytearray(32) + struct.pack('<II', 2, 5) + ent(364.1, 194.1, 25.0, 'Quant', 50.0, 10.0, 22.11, 20.0, 3.0) + ent(364.1, 152.1, 25.0, 'Qual', 50.0, 10.0, 22.11, 28.0, 4.0)\n",
        "smrm = bytearray(52); struct.pack_into('<I', smrm, 28, 30); struct.pack_into('<II', smrm, 36, 0, 60); struct.pack_into('<d', smrm, 44, 1.0)\n",
        "vam_shim = bytearray(612) + b'<?xml version=\"1.0\"?><SCIEX_ANALYST_ACQUISITION_METHOD><Category><Title>Pumps</Title><Parameter><Name>Total Flow</Name><Value>0.25</Value></Parameter><Parameter><Name>B Concentration</Name><Value>5.0</Value></Parameter></Category><Category><Title>Oven</Title><Parameter><Name>Temperature</Name><Value>40.0</Value></Parameter></Category><Category><Title>System Controller</Title><Table><Name>Time Program</Name><row><Field1>Time</Field1><Field2>Module</Field2><Field3>0</Field3><Field4>Parameter</Field4><Field5>Value</Field5></row><row><Field1>10.0</Field1><Field2>Pumps</Field2><Field3>0</Field3><Field4>Pump B Conc.</Field4><Field5>95.0</Field5></row><row><Field1>22.0</Field1><Field2>Controller</Field2><Field3>0</Field3><Field4>Stop</Field4><Field5>0</Field5></row></Table></Category></SCIEX_ANALYST_ACQUISITION_METHOD>'\n",
        "vam_acq = bytearray(612) + b'<?xml version=\"1.0\"?><SCIEX_ANALYST_ACQUISITION_METHOD><Category><Title>ACQUITY BSM Method</Title><Table><Name>Gradient Table</Name><row><Field1>Time</Field1><Field2>Flow Rate</Field2><Field3>%A</Field3><Field4>%B</Field4></row><row><Field1>Initial</Field1><Field2>0.300</Field2><Field3>90.0</Field3><Field4>10.0</Field4></row><row><Field1>8.00</Field1><Field2>0.300</Field2><Field3>5.0</Field3><Field4>95.0</Field4></row></Table></Category><Category><Title>ACQUITY SM Method</Title><Parameter><Name>Target Column Temperature</Name><Value>35.0</Value></Parameter></Category></SCIEX_ANALYST_ACQUISITION_METHOD>'\n",
        "step = lambda t, fl, a, b: bytes(36) + struct.pack('<H', 16) + 'New step'.encode('utf-16le') + struct.pack('<fffff', t, fl, 0.0, a, b) + bytes(16)\n",
        "cn, cu = 'Column Pressure'.encode('utf-16le'), '(psi)'.encode('utf-16le')\n",
        "ch_s = bytes(36) + struct.pack('<HIH', 1, 1, len(cn)) + cn + struct.pack('<H', len(cu)) + cu + struct.pack('<dHffI', 6.0, 0, 0.0, 5000.0, 0xffffffff)\n",
        "w2 = bytearray(126); struct.pack_into('<I', w2, 32, 1); struct.pack_into('<H', w2, 52, 56); w2[54:110] = b'01234567-89ab-cdef-0123-456789abcdef_0000000000000001234'\n",
        "dde = bytearray(32) + b''.join(struct.pack('<dddHHi', mz, 0.0, 1540.0, 1, 0, 0) for mz in (413.266, 520.1, 610.2))\n",
        "dad_off = bytearray(dad_hdr); struct.pack_into('<f', dad_off, 54, 210.0)\n",
        "tdc = bytearray(72); struct.pack_into('<d', tdc, 32, 1.0 / 38.4)\n",
        "tcal = bytearray(32) + struct.pack('<ddII', 0.001, 100.0, 0, 1) + struct.pack('<ddI', 0.001, 100.0, 0)\n",
        "idx1 = bytearray(32) + struct.pack('<IId', 0, 8, 6000.0) + b'\\x01\\x00' + struct.pack('<dd', 1500.0, 0.0) + bytes(20)\n",
        "tbl = bytearray(32) + b''.join(struct.pack('<II', s, 20) + bytes(8) for s in (1, 2, 3))\n",
        "cfb_scrivi.write({\n",
        "  'SampleSubtree/SampleIdxTable': bytes(tbl),\n",
        "  'SampleSubtree/Sample1/Idx': bytes(idx1),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADSampleHeader': bytes(dad_hdr),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADIdx': bytes(dad_idx),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADRealTimeData': bytes(dad_rtd),\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/Channel': ch_s,\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/DevRealTimeHeader': bytes(32) + struct.pack('<I', 3),\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/DevData': bytes(32) + struct.pack('<3d', 1200.0, 1250.0, 1210.0),\n",
        "  'SampleSubtree/Sample1/Wiff2SampleInfo': bytes(w2),\n",
        "  'SampleSubtree/Sample1/DDERealTimeData': bytes(dde),\n",
        "  'SampleSubtree/Sample1/TDCInfo': bytes(tdc),\n",
        "  'SampleSubtree/Sample1/TOFCalibrationData': bytes(tcal),\n",
        "  'SampleSubtree/Sample1/Itc': bytes(32) + struct.pack('<d', 0.5),\n",
        "  'SampleSubtree/Sample1/TDCStatistics': bytes(32) + struct.pack('<I', 1) + bytes(20),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/MassRangeEx/MassRangeEx': bytes(mrex),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/sMRM': bytes(smrm),\n",
        "  'MethodSubtree/Method1/DeviceMethod1/VendorAppMethod': bytes(vam_shim),\n",
        "  'SampleSubtree/Sample2/Idx': bytes(idx1),\n",
        "  'SampleSubtree/Sample2/Devices/Device_1/DAD/DADSampleHeader': bytes(dad_off),\n",
        "  'SampleSubtree/Sample2/Devices/Device_1/DAD/DADIdx': bytes(dad_idx),\n",
        "  'SampleSubtree/Sample2/Devices/Device_1/DAD/DADRealTimeData': bytes(dad_rtd),\n",
        "  'MethodSubtree/Method2/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method2/DeviceMethod1/VendorAppMethod': bytes(vam_acq),\n",
        "  'SampleSubtree/Sample3/Idx': bytes(idx1),\n",
        "  'MethodSubtree/Method3/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method3/DeviceMethod1/LCPumpStep0/LCPumpStep': step(0.0, 400.0, 95.0, 5.0),\n",
        "  'MethodSubtree/Method3/DeviceMethod1/LCPumpStep1/LCPumpStep': step(12.0, 400.0, 10.0, 90.0),\n",
        "}, out / 'ci.wiff')\n",
        "(out / 'ci.wiff.scan').write_bytes(bytes(44) + struct.pack('<2f', 1000.0, 500.0))\n",
    );
    let st = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(root())
        .arg(out)
        .status()
        .unwrap();
    assert!(st.success(), "synthetic CI wiff generator failed");
}

/// The container of the synthetic file, written to its own folder for each test (`tag`), and the length of its `.wiff.scan`.
fn synthetic(tag: &str) -> (Ole, u64) {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_ci_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    write_synthetic_ci_wiff(&out);
    let wiff_bytes = std::fs::read(out.join("ci.wiff")).unwrap();
    let scan_len = std::fs::metadata(out.join("ci.wiff.scan")).unwrap().len();
    let _ = std::fs::remove_dir_all(&out);
    (container(wiff_bytes, "ci.wiff").unwrap(), scan_len)
}

#[test]
fn synthetic_ci_dad_mrm_and_gradient() {
    let (ole, scan_len) = synthetic("dad");
    let w = Wiff::open(&ole, "ci.wiff", Some(scan_len), 1).unwrap();

    let dad = w.dad.as_ref().expect("synthetic DAD");
    assert_eq!((dad.points(), dad.channels()), (3, 5));
    assert_eq!(dad.wavelengths_nm, vec![200.0, 202.0, 204.0, 206.0, 208.0]);
    // a stop wavelength one step past the channels (210 for 5 channels of 2 nm from 200) is a header that does not add up: no DAD
    assert!(DadData::read(&ole, 2).is_none());
    assert_eq!(dad.twc_chromatogram(0).intensity, vec![15.0, 12.5, 30.0]);
    assert_eq!(
        dad.wavelength_chromatogram(202.0, None, 0)
            .unwrap()
            .intensity,
        vec![2.0, 1.5, 4.0]
    );

    let m1 = method_of_sample(&ole, 1).unwrap();
    let Kind::Mrm(tr) = &m1.experiments[0].kind else {
        panic!("expected MRM")
    };
    assert_eq!(tr.len(), 2);
    assert_eq!(
        (tr[0].q1, tr[0].q3, tr[0].name.as_str(), tr[0].ce, tr[0].dp),
        (364.1, 194.1, "Quant", Some(20.0), Some(50.0))
    );
    assert_eq!(
        (tr[1].q1, tr[1].q3, tr[1].name.as_str(), tr[1].ce, tr[1].cxp),
        (364.1, 152.1, "Qual", Some(28.0), Some(4.0))
    );
    let lc1 = m1.lc.unwrap();
    assert_eq!(
        (lc1.flow, lc1.b0, lc1.oven, lc1.run_time),
        (Some(0.25), Some(5.0), Some(40.0), Some(22.0))
    );
    assert_eq!(lc1.gradient.len(), 2);

    let lc2 = method_of_sample(&ole, 2).unwrap().lc.unwrap();
    assert_eq!(
        (lc2.flow, lc2.b0, lc2.oven, lc2.run_time),
        (Some(0.3), Some(10.0), Some(35.0), Some(8.0))
    );
    let lc3 = method_of_sample(&ole, 3).unwrap().lc.unwrap();
    assert_eq!(
        (lc3.flow, lc3.b0, lc3.run_time),
        (Some(0.4), Some(5.0), Some(12.0))
    );
}

#[test]
fn synthetic_ci_channels_dde_and_tof() {
    let (ole, _) = synthetic("tof");
    let chs = device_channels(&ole, 1);
    assert_eq!(
        (
            chs.len(),
            chs[0].name.as_str(),
            chs[0].unit.as_str(),
            chs[0].values.len()
        ),
        (1, "Column Pressure", "psi", 3)
    );
    // one survey and two dependent experiments: the records go cycle by cycle, experiments 1 and 2
    let dde = dde_records(&ole, 1, 3);
    assert_eq!(
        (dde.len(), dde[0].precursor_mz, dde[0].precursor_intensity),
        (3, 413.266, 1540.0)
    );
    let order: Vec<_> = dde.iter().map(|d| (d.cycle, d.experiment)).collect();
    assert_eq!(order, vec![(0, 1), (0, 2), (1, 1)]);
    let tof = TofSampleData::read(&ole, 1).unwrap();
    assert_eq!(tof.scan_calibration.len(), 1);
    assert_eq!(tof.itc, vec![0.5]);
    let tof_blk = [
        0xff, 0xff, 0xff, 0xff, 100, 0, 0, 0, 0, 5, 0x82, 0x7c, 200, 0xff,
    ];
    let pts = decode_tof_block(&tof_blk).unwrap();
    assert_eq!(pts, vec![(100, 5), (112, 200)]);
    let mz0 = tof.bin_to_mz(0, pts[0].0);
    // bin 100 flies for 2.6 ns, before the t0 of 100 ns of this calibration: no m/z; a later bin has one
    assert_eq!(mz0, 0.0);
    let later = tof.bin_to_mz(0, 6000);
    assert!(later > 0.0 && later.is_finite());
}
