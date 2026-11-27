# mzflow screens: wireframes (study)

Companion of `docs/studies/MZFLOW_STUDY.md`. ASCII wireframes of the screens of mzflow, with the existing component each part reuses. All names and numbers are synthetic (parent compound P, files `S01..S24`). Every number on screen comes from a Rust output that the CLI prints too; JS only draws.

## A. Start screen: scheme in the centre, feature tables on the right
```
+-- mzLab . Flow ----------------------------------------------------------------------------------+
| Project: photolysis of P (24 files, ESI+ and ESI-)        [Run all] [Pause]  [Save as template]  |
+---------------+----------------------------------------------------------+-----------------------+
| INPUTS        |  SCHEME                                         Canvas|List|Plan | FEATURE TABLES |
|               |                                                          |                       |
| Parent P      |   +----------+     +-----------+     +--------------+    | > A direct ESI+       |
|  formula auto |   | Metadata |---->| A direct  |---->| B kinetic    |    |   412 rows . 24 files |
|  [M+H]+ 252.1 |   | 24 files |     | 412 rows  |     | class        |    |   done 10:42          |
|  [edit]       |   | ! 1 warn |     | done      |     | 412 classed  |    | > A direct ESI-       |
|               |   +----------+     +-----------+     +------+-------+    |   198 rows . 24 files |
| Files     24  |        |                                   |            | > C suspect ESI+      |
|  T0 2 . bl 2  |        |           +-----------+     +-----v--------+    |   37 targets          |
|  dark 4 . S16 |        +---------->| C suspect |---->| union +      |    | * union A+B+C         |
|  [grid]       |                    | 37 targets|     | origin       |    |   431 rows  (open)    |
|               |                    | running   |     | waiting      |    |                       |
|               |                    | ####----  |     +--------------+    | [Open .mzfeat...]     |
+---------------+----------------------------------------------------------+-----------------------+
| Log: auto  window RT 0.2 min (estimated from S03)   . user  role of S07 -> dark   . agent  -           |
+--------------------------------------------------------------------------------------------------+
```
- Centre: the Flow canvas as it is (`flow-canvas.js`, `flow-shell.js`); each card gets one result line after the run ("412 rows", "1 warning").
- Left: the parent card (new, PARENT-CARD) and a summary of the metadata grid (`flow-meta-grid.js`), with the warnings of `samples/validate.rs`.
- Right: the list of feature tables (new, FLOW-TABLE-LIST): one entry per table output of the runs (OPFS `mzlab/flow/live/`) or opened from disk; name = block + polarity, then rows, files, state, time. The `*` marks the table the user marked as final.
- Bottom: the log with the actor of each choice (`auto`, `user`, `agent`; AUDIT-ACTOR).

## B. Each step, while it runs and after
| step | while it runs (existing live card) | after: what the card and the table entry show |
|---|---|---|
| metadata | - | counts per role and condition; warnings `noT0`, `noBlank` as yellow marks, never a block; click opens the grid |
| A direct | per-file table: state, share, time, memory (`flow-live-card.js`) | rows, files, the estimated tolerances and RT window (logged as `auto`), a 3-XIC preview (Pipeline preview, `32_pipeline*.js`) |
| B kinetic class | progress per row block | a bar of the class counts (bell, rising, decreasing, background); rows are never removed |
| C suspect | per-target progress | targets found / forced / absent (states of `quant.targets`) |
| union + origin | - | rows per origin (A only, C only, A+C ...), conflicts of `union/1` listed |
| link (3 pillars) | per-row progress | rows with 0-3 pillars, as a small histogram |

Example of a finished card and its table entry:
```
+--------------------------+        > B kinetic class ESI+    412 rows
| B kinetic class    done  |          bell 61 | rising 88 | decreasing 23 | background 240
| bell 61  rising 88       |          parameters: max/T0 >= 3, max/blank >= 3 (defaults)
| decr 23  backgr 240      |
+--------------------------+
```

## C. Feature table
```
+-- union A+B+C . ESI+ . 431 rows ------------------------------------- [Columns] [Export view] --+
| Filter: [origin: any v] [class: bell, rising] [MS2 linked] [level <= 3] [not reviewed]  search _ |
+----+---------+------+-------+--------+---------+--------+--------+------+-----+-----+------+-----+
| id | m/z     | RT   | pol   | origin | class   | max/T0 | max/bl | t/dk | MS2 | UV  | lvl  | rev |
+----+---------+------+-------+--------+---------+--------+--------+------+-----+-----+------+-----+
| 17 | 268.1   | 5.42 | +     | A,C    | bell    | 41     | 22     | 9.1  | 1   | yes | 3 *  | ok  |
| 52 | 234.1   | 4.90 | + -   | A      | rising  | inf    | 15     | 6.0  | 0   | -   | 5    | -   |
| 88 | 286.1   | 6.10 | +     | C      | bell    | 12     | 3.1    | 1.2  | 2   | no  | 4 a  | -   |
+----+---------+------+-------+--------+---------+--------+--------+------+-----+-----+------+-----+
  a = proposed by the program (outlined); * = set by the user; inf = absent at T0 (shown, not hidden)
```
Reuses `TPTAB` (`TP_Mine/js/42_tp_table.js`: sort, chips, column menu, virtual rows, export of the view) and the state and level legend of `flow-run-end.js`. New: the `origin` and `class` columns (ROW-ORIGIN, KINETIC-CLASS), the `t/dk` ratio.

## D. Feature view: one row, slot by slot
```
+-- Feature 17 . m/z 268.1 . RT 5.42 min . ESI+ . origin A,C . bell -------- [<] [>] [Card] [x] --+
| Structure                    | Peak / XIC (same scale)        | Kinetics (per condition)         |
|  [2D drawing on parent P]    |  /\    treated (colour = time) |  area o--o--o   height x--x--x   |
|  P +O, site uncertain (3)    | /  \   blank - - - dark . . .  |  photolysis  photocat  dark     |
|  [Draw] [accept] [discard]   |  apex 5.42, S/N 85, FWHM 9 s   |  tmax 60 min, t1/2 -             |
|  level [3 v] note ____       |                                |  not acquired at 240 min: gap    |
+------------------------------+--------------------------------+----------------------------------+
| MS2 (net, per polarity)      | Isotopes / adducts             | UV-Vis                           |
|  CE 25 V, apex 5.41, 4 scans |  M+1 obs 13.9 % exp 13.6 %     |  at apex - baseline              |
|  parity: even, N rule ok     |  M+2 obs 33 %  Cl possible     |  lambda max 254 nm               |
|  vs parent: cos 0.71         |  [M+Na]+ 290.1 co-elutes       |  co-elution: none flagged        |
|  losses: -H2O, -CO           |  neutral mass 267.1            |  (empty: "run dad spectrum")     |
+------------------------------+--------------------------------+----------------------------------+
| Link to parent: [MS2 0.71 ok] [RT shift earlier: more polar ok] [kinetics: rises as P decays ok] |
| Provenance: S01..S24 (SHA-256 in the card) . table union@1 . plan 3f2a... . log 4 entries        |
+--------------------------------------------------------------------------------------------------+
```
- Reuses `TPRIGA` (`TP_Mine/js/44_tp_riga.js`: XIC coloured by time with blank and dark dashed, MS1/MS2/mirror, samples), `TPSTRUCT` and `TPMOL` (structure drawn on the parent), `TPDEC` (level and note), `uv-spettro.js`, the mirror of `libreria.js`, `tdati.js` (data table of every chart, key T).
- Fixed order of slots; an empty slot says which block or CLI command fills it (FEATURE-DETAIL, `feature-slots/1`). It never shows a made-up value.
- `absent` (no peak) is a point at 0 with a hollow mark; `na` (time not acquired or file does not cover the window) is a gap in the line, never 0.
- Kinetics: replicates as the mean with SD bars and the replicate points drawn on top, `n` per point in the data table (owner's choice); area and height side by side, each normalised to its own maximum.

## E. Phone width (owner: read-only, v2, row PHONE-VIEWER)
Today a phone gets only the card "You are on a smartphone" (`telefono-scheda.js`). The read-only view adds a second button "View tables" there:
```
+------------------------+
| P photolysis    [=]    |
| Tables                 |
| > union A+B+C   431    |
| > A direct ESI+ 412    |
+------------------------+
| Feature 17  268.1 5.42 |
| bell . A,C . lvl 3     |
| [XIC] [Kin] [MS2] [UV] |
| +--------------------+ |
| |  /\  XIC           | |
| | /  \               | |
| +--------------------+ |
| < 16          18 >     |
+------------------------+
```
One slot at a time behind tabs (the `TPRIGA` charts already stack at 600 px), no runs, no editing; the scheme is not shown.

## F. Compared with MZmine (from general knowledge; not checked in its code)
| | MZmine (as we understand it) | mzflow |
|---|---|---|
| project | a project holds raw data files and feature lists; the main window shows both lists | the Flow project; tables listed on the right, files and parent on the left |
| workflow | batch mode: an ordered list of modules with their parameters, saved as XML | a graph of blocks (`mzflow/2`), live per block; also a list tab |
| feature list | one table per processing result; rows with m/z, RT, per-file shapes; a module creates a new list and keeps the old one | one table per detection run; the same: each block adds an entry, nothing is overwritten |
| feature detail | spectra and chromatogram windows opened from a row (several kinds of plot windows) | one view with fixed slots for the TP evidence |
| TP-specific evidence | not a built-in focus as far as we know | kinetics, controls, parent link, levels |
Unsure: the exact layout of the MZmine 3/4 main window, and whether its newer "wizard" shows the steps as a graph. These are not copied: only the idea "one list entry per result".
