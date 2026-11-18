# Complete Binary Specification of SCIEX `.wiff` and `.wiff.scan`

This document records the deterministic byte-by-byte binary layout of the SCIEX Analyst and SCIEX OS `.wiff` (OLE2 Compound File) and `.wiff.scan` formats across all four areas verified against the 32 `.wiff` files in `mzlab-dati/`:

1. **Optical DAD/PDA Detector** (`DADSampleHeader`, `DADIdx`, `DADRealTimeData`, `Agilent1100DTMethod`).
2. **Fine MRM and MS Experiment Metadata** (`ExperimentHeader`, `MassRangeEx`, `sMRM`, `CompoundDepParams`).
3. **Instrument Status, Analog Logs & HPLC Gradients** (`VendorAppMethod`, `LCPumpStep`, `DeviceTable`, `Channel`, `DevRealTimeHeader`, `DevData`).
4. **High-Resolution TOF / SCIEX OS & IDA Streams** (`Idx`, `DDERealTimeData`, `Wiff2SampleInfo`, `TDCInfo`, `TDCStatistics`, `TOFCalibrationData`, `Itc`, `ExperimentTOF`, `ExperimentTOFEx`, and `.wiff.scan` blocks).

All multi-byte integers (`u16`, `u32`, `i32`) and IEEE-754 floats (`f32`, `f64`) are **little-endian**. Every OLE stream in `SampleSubtree` and `MethodSubtree` begins with a standard **32-byte OLE stream header** (`+0..+32`), where `+4` (`u32`) is the header version (`4` in `SampleSubtree`, `5` in `MethodSubtree`).

---

## 1. Optical DAD / PDA Detector Streams

Located under `SampleSubtree/SampleN/Devices/Device_1/DAD/` (or `Device_K/DAD/`).

### 1.1 `DADSampleHeader` (86 bytes)

| Offset | Type | Field | Physical Meaning & Invariants |
|---|---|---|---|
| `+0..+32` | `[u8; 32]` | `ole_header` | Standard 32-byte OLE header (`u32` at `+4` = `4`, `u32` at `+28` = `1`). |
| `+32` | `u16` | `reserved_0` | Always `0`. |
| `+34` | `u32` | `n_points` | Number of time points $N_t$ (e.g. `5000` for $20\text{ min}$, `5500` for $22\text{ min}$ at $4.1667\text{ Hz}$, $\Delta t = 0.004\text{ min} = 0.24\text{ s}$). |
| `+38` | `u32` | `n_channels` | Number of wavelength channels $N_\lambda$ (e.g. `201`). |
| `+42` | `u32` | `reserved_1` | Always `0`. |
| `+46` | `f32` | `duration_min` | Total acquisition time in minutes (e.g. `19.99984` or `21.99984`). |
| `+50` | `f32` | `wl_start_nm` | Start wavelength $\lambda_{\text{start}}$ in $\text{nm}$ (e.g. `200.0`). |
| `+54` | `f32` | `wl_stop_nm` | Stop wavelength $\lambda_{\text{stop}}$ in $\text{nm}$ (e.g. `400.0`). |
| `+58` | `f32` | `wl_step_nm` | Wavelength step $\Delta\lambda$ in $\text{nm}$ (e.g. `1.0`). Invariant: $\text{round}((\lambda_{\text{stop}} - \lambda_{\text{start}})/\Delta\lambda) + 1 = N_\lambda$. |
| `+62..+86` | `[u8; 24]` | `padding` | 24 zero bytes. |

### 1.2 `DADIdx` ($32 + 8 \times N_t$ bytes)

Contains the retention time axis and the pre-summed Total Wavelength Chromatogram (TWC):

| Offset | Type | Field | Physical Meaning & Invariants |
|---|---|---|---|
| `+0..+32` | `[u8; 32]` | `ole_header` | Standard 32-byte OLE header (`u32` at `+4` = `4`). |
| `+32 + 8*i` | `f32` | `time_min[i]` | Retention time in minutes for point $i \in \{0 \dots N_t - 1\}$ ($t_i = (i + 1) \times 0.004\text{ min}$). |
| `+36 + 8*i` | `f32` | `twc_mau[i]` | Total Wavelength Chromatogram intensity in $\text{mAU}$: $\text{twc}[i] = \sum_{j=0}^{N_\lambda-1} M[i, j]$ within `f32` rounding ($\le 1.3 \times 10^{-3}\text{ mAU}$). |

### 1.3 `DADRealTimeData` ($32 + 4 \times N_\lambda \times N_t + \text{zero tail}$ bytes)

| Offset | Type | Field | Physical Meaning & Invariants |
|---|---|---|---|
| `+0..+32` | `[u8; 32]` | `ole_header` | Standard 32-byte OLE header (`u32` at `+4` = `4`). |
| `+32..+32 + 4*N_lambda*N_t` | `[[f32; N_lambda]; N_t]` | `matrix_mau` | Row-major 2D absorbance matrix $M[i, j]$ in $\text{mAU}$, where row $i \in \{0 \dots N_t - 1\}$ is time `DADIdx.time_min[i]` (stride $4 \times N_\lambda = 804\text{ B}$ for 201 channels) and column $j \in \{0 \dots N_\lambda - 1\}$ is wavelength $\lambda_j = \lambda_{\text{start}} + j \times \Delta\lambda$. |
| `+32 + 4*N_lambda*N_t..EOF` | `[u8; *]` | `zero_tail` | Pre-allocated OLE sector/buffer slack (e.g. `11492` zero bytes on 5000-pt streams, `2736` zero bytes on 5500-pt streams; 100% `0x00`). |

### 1.4 `Agilent1100DTMethod` (216 bytes, in `MethodSubtree/MethodN/DeviceMethod2/`)

- `+0..+32`: OLE header (`u32` at `+4` = `5`).
- `+36` (`u16`): `name_byte_len` (`20`) followed by UTF-16LE `"New method"` (`+38..+58`).
- `+72` (`f32`): `100.0` (reference/scale).
- `+76` (`f32`): `wl_start_nm` (`200.0`), `+80` (`f32`): `wl_stop_nm` (`400.0`), `+84` (`f32`): `wl_step_nm` (`1.0`).

---

## 2. Fine MRM and MS Experiment Metadata

Located under `MethodSubtree/MethodN/DeviceMethod0/PeriodM/ExperimentK/`.

### 2.1 `ExperimentHeader` (180 bytes)

| Offset | Type | Field | Physical Meaning |
|---|---|---|---|
| `+0..+32` | `[u8; 32]` | `ole_header` | Standard 32-byte OLE header (`u32` at `+4` = `5`). |
| `+42` | `f64` | `fixed_mz` | Precursor or fixed $m/z$ for Product Ion (`11`), Precursor Ion (`5`), Neutral Loss (`7`), TOF MS/MS (`9`); `0.0` for MRM (`4`) and Full Scan (`15`). |
| `+86` | `u32` | `polarity` | `0` = positive (`+1`), `1` = negative (`-1`). |
| `+90` | `u16` | `exp_name_len` | Byte length of experiment name in UTF-16LE (e.g. `28` for `"New experiment"` at `+92..+120`). |
| `+120` | `u16` | `mrm_marker` | `0xffff` on 3200 QTRAP MRM, `0x0000` otherwise. |
| `+122` | `u32` | `scan_type` | `4` = MRM, `5` = Precursor Ion, `7` = Neutral Loss, `8` = TOF MS, `9` = TOF MS/MS, `11` = Product Ion / EPI, `15` = Full Scan / EMS. |
| `+176` | `u32` | `n_entries` | Exact number of mass ranges or MRM transitions in `MassRangeEx/MassRangeEx` (verified on 422/422 experiments). |

### 2.2 `MassRangeEx/MassRangeEx` (Variable length, deterministic grammar)

Total size is $40 + \sum_{e=0}^{N_{\text{entries}}-1} \text{sizeof}(\text{Entry}_e)$ with zero trailing slack across all 422 experiments.

#### Stream Header (40 bytes)
- `+0..+32`: 32-byte OLE header (`u32` at `+4` = `5`).
- `+32` (`u32`): format version (`2`).
- `+36` (`u32`): `n_params` — number of lens/voltage parameter blocks per entry:
  - `5` on 3200 QTRAP MRM (`["DP", "EP", "CEP", "CE", "CXP"]`).
  - `4` on QTRAP 4000/4500/5500/6500 MRM (`["DP", "EP", "CE", "CXP"]`).
  - `9` on 3200 QTRAP Scan (`["DP", "EP", "CEP", "CE", "AF2", "AF3", "EXB", "C2B", "CES"]`).
  - `4` on X500R / ZenoTOF 7600 (`["DP", "CE", "DPS", "CES"]`).

#### Entry Structure (repeated `ExperimentHeader.n_entries` times starting at `+40`)

1. **Entry Header (`22 + name_byte_len` bytes)**:
   - `+0` (`f32`): `m0` — in MRM (`scan_type == 4`): **Q1 precursor $m/z$**; in Scan: **start $m/z$**.
   - `+4` (`f32`): `m1` — in MRM: `0.0`; in Scan: **stop $m/z$**.
   - `+8` (`f32`): `m2` — in MRM: **Q3 product $m/z$**; in Scan: **step $m/z$** (e.g. `0.06` Da).
   - `+12` (`f32`): `dwell_or_rt` — in unscheduled MRM: **dwell time in $\text{ms}$** (e.g. `25.0`); in scheduled MRM (`sMRM.is_scheduled == 1`): **target retention time in $\text{min}$**; in Scan: scan/step time in $\text{s}$.
   - `+16` (`u32`): reserved (`0`).
   - `+20` (`u16`): `name_byte_len` — byte length of the UTF-16LE transition label (`0` when unnamed; `10` for `"Quant"`, `8` for `"Qual"`).
   - `+22..+22 + name_byte_len`: UTF-16LE transition label (`name`).

2. **Parameter Array (`n_params` blocks immediately following the label)**:
   Each parameter block has length $2 + \text{plen} + 12$ bytes:
   - `+0` (`u16`): `plen` — byte length of the UTF-16LE parameter ID (`4` for `"DP"`, `"EP"`, `"CE"`; `6` for `"CEP"`, `"CXP"`, `"CES"`).
   - `+2..+2 + plen`: UTF-16LE parameter ID (`"DP"`, `"EP"`, `"CEP"`, `"CE"`, `"CXP"`, etc.).
   - `+2 + plen` (`f32`): `val_start` — parameter voltage in $\text{V}$ / $\text{eV}$.
   - `+6 + plen` (`f32`): `val_stop` — stop voltage (equals `val_start` unless ramped).
   - `+10 + plen` (`f32`): `val_step` — ramp step (`0.0` unless ramped).

#### Exact Offset Map in `MRM.wiff` (290 bytes total, 2 transitions, `n_params = 5`)

- **Entry 0 (`+40..+166`, 126 bytes — `"Quant"`)**:
  - `+40` (`f32`): `Q1 = 364.1`, `+44` (`f32`): `0.0`, `+48` (`f32`): `Q3 = 194.1`, `+52` (`f32`): `dwell = 25.0` ms, `+56` (`u32`): `0`, `+60` (`u16`): `10`, `+62..+72`: `"Quant"`.
  - `+72..+90`: `"DP"` (`f32` at `+78` = `50.0` V), `+90..+108`: `"EP"` (`f32` at `+96` = `10.0` V), `+108..+128`: `"CEP"` (`f32` at `+116` = `22.10639` V), `+128..+146`: `"CE"` (`f32` at `+134` = `20.0` V), `+146..+166`: `"CXP"` (`f32` at `+154` = `3.0` V).
- **Entry 1 (`+166..+290`, 124 bytes — `"Qual"`)**:
  - `+166` (`f32`): `Q1 = 364.1`, `+170` (`f32`): `0.0`, `+174` (`f32`): `Q3 = 152.1`, `+178` (`f32`): `dwell = 25.0` ms, `+182` (`u32`): `0`, `+186` (`u16`): `8`, `+188..+196`: `"Qual"`.
  - `+196..+214`: `"DP"` (`f32` at `+202` = `50.0` V), `+214..+232`: `"EP"` (`f32` at `+220` = `10.0` V), `+232..+252`: `"CEP"` (`f32` at `+240` = `22.10639` V), `+252..+270`: `"CE"` (`f32` at `+258` = `20.0` V), `+270..+290`: `"CXP"` (`f32` at `+278` = `3.0` V).

### 2.3 `sMRM` (52 bytes)

| Offset | Type | Field | Physical Meaning |
|---|---|---|---|
| `+0..+28` | `[u8; 28]` | `ole_header` | OLE header (`u32` at `+4` = `5`, `u32` at `+16` = `0x1e72`). |
| `+28` | `u32` | `default_dwell_ms` | `30` on Analyst, `0` on SCIEX OS. |
| `+32` | `u32` | `reserved` | Always `0`. |
| `+36` | `u32` | `is_scheduled` | `1` = Scheduled MRM (`sMRM`, so `MassRangeEx.dwell_or_rt` is target RT in minutes); `0` = unscheduled MRM. |
| `+40` | `u32` | `window_s` | MRM detection window in seconds (e.g. `60`, `120`, `180`, `360`). |
| `+44` | `f64` | `target_scan_time_s` | Target cycle time in seconds (e.g. `1.0`). |

---

## 3. Instrument Status Streams, Analog Logs & HPLC Gradients

### 3.1 `VendorAppMethod` (`MethodSubtree/MethodN/DeviceMethod1/VendorAppMethod`)

- **Binary Preamble (`+0..xml_offset`)**:
  - Fixed variant (`612` bytes): `+0..+32` OLE header (`u32` at `+4` = `5`), `+604` (`u32`) = `250` (`0xfa`), `+608` (`u32`) = `xml_byte_len`, `+612..`: UTF-8 XML payload `<?xml version="1.0"...`.
  - Variable `.ini`-path variant (`620..810` bytes, e.g. Waters Acquity / Analyst driver): embeds a UTF-16LE driver path (`C:\Program Files\...\AcquityUPLC.ini`) followed by `fa 00 00 00` and `<u32 xml_byte_len>` immediately before `<?xml`.
- **Internal XML Grammars (`<SCIEX_ANALYST_ACQUISITION_METHOD>`)**:
  1. **Shimadzu LC + PDA**:
     - `<Category><Title>Pumps</Title>`: `<Parameter><Name>Total Flow</Name><Value>...</Value>`, `<Name>B Concentration</Name>`, `<Name>P.Max 1</Name><Value>...</Value><Units>...</Units>`.
     - `<Category><Title>Oven</Title>`: `<Parameter><Name>Temperature</Name><Value>...</Value>`.
     - `<Category><Title>System Controller</Title><Table><Name>Time Program</Name>`: Row 0 is headers; subsequent `<Row>` elements carry `Field1` = time ($\text{min}$), `Field2` = module (`"Pumps"` / `"Controller"`), `Field4` = action (`"Pump B Conc."`, `"B.Conc"`, `"Total Flow"`, `"T.Flow"`, `"Stop"`), `Field5` = numeric value.
     - `<Category><Title>PDA Detector</Title>`: `Start WL`, `Stop WL`, `Wave Step`, `Sampling Frequency`, `Lamp`, `Cell Temperature`, `Slit Width`, plus `<Table><Name>DA Channel Array</Name>`.
  2. **Waters Acquity UPLC**:
     - `<Category><Title>ACQUITY BSM Method</Title><Table><Name>Gradient Table</Name>`: Row 0 is column titles (`Time`, `Flow Rate`, `%A`, `%B`, `Curve`); Row 1 has `Field1="Initial"` ($t = 0.0\text{ min}$); subsequent rows have `Field1` ($t\text{ min}$), `Field2` (flow in $\text{mL/min}$), `Field3` ($\%A$), `Field4` ($\%B$), `Field5` (curve).
     - `<Category><Title>ACQUITY SM Method</Title>`: `<Parameter><Name>Target Column Temperature</Name><Value>...</Value>`.

### 3.2 Binary `LCPumpStep` (`MethodSubtree/MethodN/DeviceMethod1/LCPumpStepK/LCPumpStep`, 90 bytes)

Fallback binary LC gradient table when `VendorAppMethod` is absent:
- `+0..+32`: OLE header (`u32` at `+4` = `5`).
- `+36` (`u16`): `16`, `+38..+54`: UTF-16LE `"New step"`.
- `+54` (`f32`): `time_min` ($t$ in $\text{min}$).
- `+58` (`f32`): `flow_ul_min` (flow rate in $\mu\text{L/min}$; divide by $1000$ for $\text{mL/min}$).
- `+66` (`f32`): `pct_a` ($\%A$).
- `+70` (`f32`): `pct_b` ($\%B$).

### 3.3 Real-Time Analog / Pressure / Temperature Logs (`SampleSubtree/SampleN/Devices/`)

- **`DeviceTable`**: 38 bytes when empty (`u32` at `+32` = `0`); when populated, `+36` (`u16`) = `n_devices`, followed by records containing UTF-16LE `"DriverName&ModelName"` (e.g. `"Shimadzu Lc&10"`, `"Acquity&UPLC"`, `"Agilent1260&1"`).
- **`Device_K/Channel`**:
  - `+0..+32`: OLE header (`u32` at `+4` = `4`).
  - `+32` (`u32`): `0`, `+36` (`u16`): `n_channels` ($N_c$).
  - Starting at `+38`, repeated $N_c$ times to exact EOF:
    - `+0` (`u32`): `enabled_flag` (`1` or `0`).
    - `+4` (`u16`): `name_byte_len`, followed by UTF-16LE channel name (e.g. `"Column Pressure"`, `"Pump A Flowrate"`, `"System Pressure"`, `"Oven: Temperature"`).
    - `+6 + name_byte_len` (`u16`): `unit_byte_len`, followed by UTF-16LE unit (e.g. `"(psi)"`, `"bar"`, `"kPa"`, `"°C"`).
    - Followed by a 22-byte trailer:
      - `+0` (`f64`): `dt_s` — sampling interval in seconds (e.g. `1.0`, `2.0`, `6.0`).
      - `+8` (`u16`): `channel_idx` ($0 \dots N_c - 1$).
      - `+10` (`f32`): `scale_min`, `+14` (`f32`): `scale_max`, `+18` (`u32`): `0xffffffff`.
- **`Device_K/DevRealTimeHeader` (36 bytes)**:
  - `+32` (`u32`): `total_values` ($N_{\text{tot}} = N_{\text{pts}} \times N_c$).
- **`Device_K/DevData` ($32 + 8 \times N_{\text{tot}}$ bytes)**:
  - `+32..`: interleaved `f64` array `[[f64; N_c]; N_pts]` where `val[k][c]` is the reading of channel $c$ at time $t_k = (k \times \text{dt\_s}) / 60.0\text{ min}$.

---

## 4. High-Resolution TOF / SCIEX OS & IDA Streams

### 4.1 `Idx` (`SampleSubtree/SampleN/Idx`, $32 + 54 \times N_{\text{scans}}$ bytes)

Each 54-byte record $i \in \{0 \dots N_{\text{scans}} - 1\}$ (where $\text{cycle} = i / E$, $\text{exp} = i \bmod E$):

| Offset | Type | Field | QTRAP Meaning | TOF / SCIEX OS Meaning |
|---|---|---|---|---|
| `+0` | `u32` | `offset` | Byte offset in `.wiff.scan` section. | Byte offset in `.wiff.scan` section. |
| `+4` | `u32` | `size` | Block size in bytes (`0` if empty). | Block size in bytes (`0` if untriggered IDA slot). |
| `+8` | `f64` | `time_ms` | Retention time in $\text{ms}$. | Retention time in $\text{ms}$ (`0.0` if untriggered). |
| `+16` | `u16` | `scan_flags` | `1` (QTRAP scan). | `6` (TOF scan; `0` if untriggered). |
| `+18` | `f64` | `tic` | Total Ion Current ($\sum y_k$). | Total Ion Current ($\sum c_k$, exact integer sum). |
| `+26` | `f64` | `aux_26` | Point count $N$. | Base Peak Intensity ($\max_k c_k$, exact integer max). |
| `+34` | `f64` | `aux_34` | `0.0`. | Hardware pulse weight $W$ ($\text{Itc}[i] \times W \approx 2.0$ MS1, $0.1$ MS2). |
| `+42` | `u32` | `zero_42` | `0`. | `0`. |
| `+46` | `f32` | `aux_46` | `0.0`. | Base peak flight time coordinate ($\approx 10.75\text{--}11.15$). |
| `+50` | `u32` | `zero_50` | `0`. | `0`. |

### 4.2 `DDERealTimeData` ($32 + 32 \times N_{\text{cycles}} \times (E - 1)$ bytes)

Stores one **32-byte record** for every dependent MS2 scan slot $k = \text{cycle} \times (E - 1) + (\text{exp} - 1)$ for $\text{exp} \in \{1 \dots E - 1\}$ (verified exact byte size across all 18 IDA/DDA samples):

| Offset | Type | Field | Physical Meaning |
|---|---|---|---|
| `+0` | `f64` | `precursor_mz` | Dynamically selected IDA precursor $m/z$ (`0.0` when slot not triggered). Matches the triggered MS1 peak centroid to $< 0.001\text{ Da}$. |
| `+8` | `f64` | `isolation_or_idx` | `0.0` on SCIEX OS / TripleTOF; on Analyst QTRAP IDA, `+8` (`u32`) is the 1-based trigger index and `+12` (`u32`) is charge/flag (`1`). |
| `+16` | `f64` | `precursor_intensity` | Exact apex detector count of the triggered precursor peak in the preceding MS1 survey scan (verified 100% identical to MS1 `apex_cnt` on SCIEX OS / TripleTOF). |
| `+24` | `u16` | `charge_state` | Assigned precursor charge state (`0` if undetermined). |
| `+26` | `u16` | `dde_flags` | Dynamic exclusion / trigger flags (`0`). |
| `+28` | `i32` | `reserved` | `0`. |

### 4.3 `Wiff2SampleInfo` / `WiffNSampleInfo` (126 bytes)

Links a sample $N$ in `.wiff` to its companion `.wiff2` / SCIEX OS acquisition record:
- `+0..+32`: OLE header (`u32` at `+4` = `4`).
- `+32` (`u32`): `sample_number` ($1 \dots N$, 1-based).
- `+36..+52` (`[u8; 16]`): `sample_guid` (Microsoft mixed-endian GUID).
- `+52` (`u16`): `id_str_len` (`56` = `0x38`).
- `+54..+110` (`[u8; 56]`): ASCII string `"<sample_guid>_<64bit_timestamp_id>"`.
- `+110..+126` (`[u8; 16]`): `batch_guid` (Microsoft mixed-endian GUID).

### 4.4 `TDCInfo` (72 bytes), `TDCStatistics`, `TOFCalibrationData`, `Itc`, `ExperimentTOF`

- **`TDCInfo` (72 B)**: `+32` (`f64`) = `tdc_bin_time_ns` $= 0.026041666666666668\text{ ns} = 1 / 38.4\text{ GHz}$ (fundamental TDC tick period; 1 TOF bin $= 4\text{ ticks} = 0.10416667\text{ ns}$). `+40..+72` = 32 zero bytes.
- **`TDCStatistics` ($32 + 4 + 20 \times N_{\text{stat}}$ B)**: `+32` (`u32`) = `n_stat`, followed by `n_stat` 20-byte per-scan TDC pulse/dead-time records.
- **`TOFCalibrationData` ($32 + 24 + 20 \times N_{\text{scans}}$ B)**:
  - `+32` (`f64`): `initial_a` (global slope $a_0$), `+40` (`f64`): `initial_t0` (global offset $t_0$), `+48` (`u32`): `0`, `+52` (`u32`): `n_scans`.
  - `+56 + 20*i`: per-scan record (`+0` `f64` `cal_a`, `+8` `f64` `cal_t0`, `+16` `u32` `flags`).
  - **TOF Mass Calibration Law**:
    $$m/z = \bigl((\text{bin} \times \Delta t_{\text{TDC}} - t_0) \times a\bigr)^2, \qquad \Delta t_{\text{TDC}} = \text{TDCInfo}[+32] = \frac{1}{38.4}\text{ ns}$$
- **`Itc` ($32 + 8 \times N_{\text{scans}}$ B)**: `+32 + 8*i` (`f64`) = `itc[i]` (Ion Transmission Control scaling factor for scan $i$).
- **`ExperimentTOF` (38 B) & `ExperimentTOFEx` (126 B)**: `+32` (`f32` `11.0` in `ExperimentTOF`, `u32` `11` in `ExperimentTOFEx`).

### 4.5 `.wiff.scan` TOF Block Format (`0xffffffff` marker)

For each non-empty TOF scan $i$, at `marker_pos = SampleIdxTable.offset + 24 + Idx[i].offset` (`section_base + 44 + Idx[i].offset`):
1. **Preceding Protobuf Header** (between previous block end and `marker_pos`): varint length followed by Field 1 (`{1: f64 cal_a, 2: f64 cal_t0}`) and repeated Field 2 seek checkpoints (`{1: varint bin, 2: varint block_offset}` every ~512 bytes).
2. **TDC Block (`Idx[i].size` bytes at `marker_pos`)**:
   - `+0` (`u32`): `0xffffffff` (`TOF_START`).
   - `+4` (`u32`): `first_bin` (initial TDC tick index, multiple of 4).
   - `+8` (`u8`): `0x00`.
   - `+9..size`: Bytecode stream advancing `curr_bin` by 4 ticks per bin:
     - `0x81..=0xfb`: skip $z = b - 0x80$ empty bins (`curr_bin += 4 * z`).
     - `0xfc` + `u8 z`, `0xfd` + `u16 z`, `0xfe` + `u32 z`: skip $z$ empty bins (`curr_bin += 4 * z`).
     - `0x01..=0x7b`: emit point at `curr_bin` with count $c = b$, then `curr_bin += 4`.
     - `0x7c` + `u8 c`, `0x7d` + `u16 c`, `0x7e` + `u32 c`: emit point at `curr_bin` with count $c$, then `curr_bin += 4`.
     - `0xff`: end-of-block padding up to `size`.
   - **Exact Invariants**: $\sum c_k = \text{Idx}[i].\text{tic}$ and $\max_k c_k = \text{Idx}[i].\text{aux\_26}$ across 100% of scans.
