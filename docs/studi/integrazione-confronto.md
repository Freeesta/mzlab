# Manual peak integration: how other packages do it, and a single model for mzLab

Study for Federico (PhD student in chemistry, Torino). He integrates low-resolution SIM/SRM traces from triple quadrupoles such as the QTRAP 3200 by hand. Today an mzLab integration has a window, drawn as two vertical bars with square handles, and one of four baseline modes («da zero», «linea tra i bordi», «linea personalizzata», «finestra fino a zero»). He finds the bars bulky and confusing, because moving a bar changes the area in a way that is hard to read. He would like only two points on the chromatogram, each of which can follow the curve, go to zero or be placed by hand.

This document has four parts:
- how the main vendor and open-source packages place boundaries and baselines;
- how each package handles overlapping peaks, signal below the baseline, propagation of a window to other samples, and auditing;
- the pharmacopoeia conventions;
- a proposal for mzLab, with implementation steps and open questions.

It describes features only and copies no vendor text beyond short quoted phrases. Product names are trademarks of their owners. A parallel chat (INTEGRA-KEEP) is making small interface fixes: several integrations per chart, no vertical bars, draggable baseline points, copy-on-click. This study changes no code.

## Sources
Each fact below carries a key. **✓** means a source page was read for this study. **(u)** means it was not verified: it comes from general knowledge, from a search-result snippet, or from a page title only. Treat it as a lead.
- **[XC]** Thermo, *Xcalibur Acquisition and Processing User Guide* XCALI-97209 rev. A (2009), Concordia University mirror, sections "Setting Genesis/ICIS/Avalon Detection Parameters" and "Avalon Event List Dialog Box" (Table 24), "Genesis/ICIS advanced parameters" (Table 32): https://www.concordia.ca/content/dam/artsci/research/cbams/docs/XCAL_AcqProc_UG.pdf ✓. The newer Xcalibur 3.1/4.1 Quan and TraceFinder manuals on `documents.thermofisher.com` refused automated download.
- **[OL]** Agilent OpenLab CDS online help (ChemStation integrator):
  - initial events: https://openlab.help.agilent.com/2.8/en/mergedProjects/DataAnalysis/11137895563-1-54043206835275531-7.htm
  - advanced events: https://openlab.help.agilent.com/en/mergedProjects/DataAnalysis/iDA_DataAnalysis_Method_IntegrationEventsCS_Advanced.htm
  - peak-to-valley ratio: https://openlab.help.agilent.com/en/mergedProjects/DataAnalysis/11016516107-1-126100800578256523-15.htm
  - skim criteria: https://openlab.help.agilent.com/en/mergedProjects/DataAnalysis/11016525067-1-126100800578256523-15.htm
  - tangent skim modes: https://openlab.help.agilent.com/en/mergedProjects/DataAnalysis/11016552459-1-126100800578256523-15.htm
  - baseline correction modes: https://openlab.help.agilent.com/en/mergedProjects/DataAnalysis/18014409523077771-3-126100800578256523-15.htm
  - manual integration: https://openlab.help.agilent.com/2.8/en/mergedProjects/DataAnalysis/DA_Manual_Integration.htm
  
  All ✓. The timed-events page and the ChemStation Reference PDF returned HTTP 403.
- **[MHQ]** Agilent knowledge base, *Modify the integration parameters in MassHunter Quantitative Analysis*: https://community.agilent.com/knowledge/masshunter-portal/w/masshunter-knowledgebase/10068/modify-the-integration-parameters-in-masshunter-quantitative-analysis ✓. Forum thread on manual-integration reporting: https://community.agilent.com/technical/mass-spectrometry-software/f/mass-spectrometry-software-user-forum/7624/manual-integration-reporting---masshunter ✓ (community answer, not official).
- **[SK]** Skyline:
  - *Peak area calculation* tip: https://skyline.ms/home/software/Skyline/wiki-page.view?name=tip_peak_calc ✓
  - *Synchronized Integration*: https://skyline.ms/home/software/Skyline/wiki-page.view?name=Synchronized%20Integration ✓
  - DIA tutorial 24.1: https://skyline.ms/tutorials/24-1/DIA/en ✓
  - *Importing Integration Boundaries* tutorial 2.6 ✓
  - support thread *MRM integration – peak area vs. background area*: https://skyline.ms/__r3/announcements-thread.view?rowId=2874 (search snippet only)
- **[WE]** Waters Empower tips 318/319 on skimming riders with ApexTrack and with the traditional algorithm:
  - https://support.waters.com/KB_Inf/Empower_Tips_of_the_Week/WKB256409_Integrating_Peaks_Skimming_rider_peaks_off_main_peaks_-_Apex_Track_peak_algorithm ✓
  - https://support.waters.com/KB_Inf/Empower_Tips_of_the_Week/WKB256411_Integrating_Peaks_Skimming_rider_peaks_off_main_peaks_-_Traditional_algorithm ✓
  - ApexTrack brochure 720000592: https://www.waters.com/content/dam/waters/fr/library/brochures/2016/waters-brochure-EmpowerSoftwareApexTrackIntegrationAlgorithm-720000592.pdf (search snippet only)
- **[WKB]** Waters MassLynx knowledge-base titles (read as titles only, (u)):
  - WKB4274 *How to set integration parameters in QuanLynx or TargetLynx to drop a vertical line when integrating unresolved peaks*
  - WKB45310 *How to manually integrate a peak in MassLynx*
  - WKB509 *TargetLynx not saving peak modifications*
  - WKB620 *How to apply specific integration parameters to individual samples*
  - WKB88347 *Integration Window Extent*
  
  All under https://support.waters.com/KB_Inf/MassLynx/. The TargetLynx application note 720001733 returned HTTP 503 and was not read.
- **[SX]** SCIEX knowledge base:
  - *Adjusting integration parameters to exclude peak tailing area in Analyst* (noise percentage): https://sciex.com/support/knowledge-base-articles/adjusting-integration-parameters-to-exclude-peak-tailing-area-in-analyst_en_us ✓ (partly a fetch-tool summary)
  - *What does a results table's Signal/Noise column in SCIEX OS show*: https://sciex.com/support/knowledge-base-articles/what-is-the-signal-noise-column-from-the-sciex-os-result-table-show-us_en_us (snippet)
  - *What are the integration algorithms used by SCIEX OS*: https://www.sciex.com/support/knowledge-base-articles/what-are-the-integration-algorithms-used-by-sciex-os_en_us (snippet)
  - *Manual integration percentage rule*: https://www.sciex.com/support/knowledge-base-articles/how-to-set-manual-integration-percentage-rule-in-sciex-os-software_en_us (snippet)
  
  The MultiQuant and SCIEX OS reference guides were not reachable, so the MQ4 parameter definitions below are (u).
- **[MZ]** mzmine documentation (version 4):
  - local minimum resolver: https://mzmine.github.io/mzmine_documentation/module_docs/featdet_resolver_local_minimum/local-minimum-resolver.html ✓
  - gap filling: https://mzmine.github.io/mzmine_documentation/module_docs/gapfill_peak_finder/gap-filling.html ✓
  - join aligner ✓
  - feature detection results (feature state colours) ✓
- **[XM]** xcms reference (rdrr.io mirror, 2020 build):
  - findChromPeaks-centWave: https://rdrr.io/bioc/xcms/man/findChromPeaks-centWave.html ✓
  - matchedFilter ✓
  - fillChromPeaks ✓
  - adjustRtime-peakGroups ✓
  - xcms source `do_findChromPeaks-functions.R` (definitions of `into`/`intb`, snippet)
- **[OC]** OpenChrom: GitHub README and wiki home only. No integrator documentation was found. Everything about OpenChrom is (u).
- **[PH]** Pharmacopoeias:
  - S/N window history, gmp-compliance.org news: https://www.gmp-compliance.org/gmp-news/revision-of-usp-chapter-621-chromatography-concerning-signal-to-noise-ratio and https://www.gmp-compliance.org/gmp-news/further-revision-of-ph-eur-general-chapter-2-2-46-concerning-signal-to-noise-ratio
  - Waters notice WKB254747 on USP <621> S/N
  - Ph. Eur. 2.2.46 text as reproduced in BP 2013: https://www.drugfuture.com/Pharmacopoeia/BP2013/data/993.html
  
  These are snippets. The USP page returned HTTP 403 and the ICH page is rendered by JavaScript.
- **[FDA]** FDA, *Data Integrity and Compliance With Drug CGMP – Questions and Answers*, Guidance for Industry, December 2018: https://www.fda.gov/media/119267/download ✓ (PDF text read).
- **[LC]** LCGC:
  - J. W. Dolan, "Integration problems", LC Troubleshooting: https://chromatographyonline.com/view/integration-problems
  - Integration Errors in Chromatographic Analysis, Part I (peaks of approximately equal size): https://chromatographyonline.com/view/integration-errors-chromatographic-analysis-part-i-peaks-approximately-equal-size
  - Part II (large peak size ratios): https://chromatographyonline.com/view/integration-errors-chromatographic-analysis-part-ii-large-peak-size-ratios
  - When Peaks Collide, Part III: https://www.chromatographyonline.com/view/when-peaks-collide-part-iii
  
  All are search snippets: the conclusions are quoted from the snippets, and the full texts were not read.

mzLab side: `docs/agenti/dati-integrazione.md` (current modes, `peak_area`, `intwin::move_end`, bars and ends) and `docs/studi/INTEGRAZIONE_PICCHI.md` (automatic HR integration, bench).

## 1. Survey table
Column meanings:
- "Boundaries" is how the user sets the start and end.
- "Baseline" lists the baseline options.
- "Below" is what happens to signal under the baseline inside the window.
- "Spread" is how a window reaches other samples.
- "Audit" is how a manual edit is marked.

| Package | Boundaries | Baseline | Overlap | Below | Spread | Audit |
|---|---|---|---|---|---|---|
| Thermo Xcalibur, Genesis / ICIS / Avalon [XC] | automatic, from the detector parameters. Manual: drag on the peak in Quan Browser (u) | straight line between start and end. Genesis/ICIS: "Baseline and Noise Window", Baseline Noise Tolerance %, "Baseline window" = number of scans searched for a local minimum ✓. Avalon: "Tension" sets how closely the baseline follows the trace ✓ | Genesis "valley detection" drops a vertical line from the valley to the baseline; Rise % and Valley S/N set when ✓. Avalon: P-P resolution (% height overlap → cluster with drop line instead of valley-to-valley), Tangent Skim event, tallest peak = parent ✓ | Avalon "Negative Peaks" event ✓. Rest (u) | per component: expected RT and window, "Nearest RT" or "Highest peak" identification ✓ | Quan Browser "Data Flags" section, flag for manual integration (u) |
| Agilent ChemStation / OpenLab CDS integrator [OL] | initial events: slope sensitivity sets start and end; peak width; area and height reject. Area and height reject "is ignored during manual integration" ✓ | baseline correction mode: Classical / No penetrations / Advanced / Advanced+ ✓. Timed events: baseline now, baseline at valleys, baseline hold (u: page 403) | peak-to-valley ratio below the set value → drop line, otherwise valley baseline ✓. Tangent skim (straight, exponential, new exponential, Gaussian, standard) with skim height ratio Hp/Hc and skim valley ratio Hc/Hv ✓. Shoulders: off / drop / tangential / exponential / Gaussian ✓ | Classical "accepts baseline penetrations". No penetrations "removes baseline penetrations by reconstructing the baseline" ✓ | manual corrections are saved with the injection, or copied to the processing method under "Manual Integration" events per signal. Injection corrections win over method ones. "Clear corrections" on reprocess ✓ | baseline codes BB/BV/VB/VV, "M" for manual (u) |
| Agilent MassHunter Quant [MHQ] | automatic per compound; manual by dragging the baseline end points (u) | integrator choice: Agile, Agile2 (parameter-less, "recommended for GC/MS"), MS-MS, General, Universal (timed events), Spectrum Summation ✓ | inside the integrator; Universal allows timed events ✓ | (u) | "Apply" (one compound) vs "Apply to All"; qualifiers use the parent's parameters by default ✓ | manual chromatograms marked "*" (forum, ✓ as a claim). Report option "original baseline check after manual integration" draws the old baseline dotted (forum) |
| Waters MassLynx / TargetLynx / QuanLynx; Empower ApexTrack [WE][WKB] | ApexTrack finds the apex from the second derivative, then the start and end from liftoff % and touchdown % (snippet). Manual: drag in the TargetLynx browser, then "Save peak modifications" [WKB509] (u) | straight baseline between the peak's start and end (u). MassLynx legacy integrator: Mean or Savitzky-Golay smoothing (u) | ApexTrack default: "drop a perpendicular at the valley point to the baseline" ✓. Events: tangential skim (main-to-rider height ratio), Gaussian skim, exponential skim. The areas "will vary depending on the chosen technique" ✓ | (u) | per compound: expected RT and window. "Integration Window Extent" [WKB88347] (u). Sample-specific parameters [WKB620] (u) | manual changes must be saved explicitly. "Save manual integrations" on reprocessing (snippet). Flag codes (u) |
| SCIEX Analyst / MultiQuant / SCIEX OS: MQ4, AutoPeak, IntelliQuan [SX] | automatic: expected RT ± RT half window. Manual: draw the baseline by drag in peak review (u) | MQ4: noise percentage, where a higher value integrates on a higher baseline and gives smaller areas, start at 40–50 %, max 99 % ✓. Baseline subtract window, peak splitting, Gaussian smooth, minimum width and height (u). MQ4 estimates the baseline from the whole chromatogram, AutoPeak at the apex (snippet) | peak splitting (points), valley handling (u) | (u) | expected RT and RT window per component; integration defaults are set before the method is created (✓ summary). "Apply to all" (u) | integration type column marks manual peaks; audit trail with reason in regulated mode; "manual integration percentage rule" (snippet). Details (u) |
| Skyline [SK] | automatic picking: mProphet or the default model. Manual: click and drag beneath the x-axis, or drag the black dashed boundary line ✓ | background height = **minimum intensity at the two boundaries**: a flat line, not sloped ✓. Background area = Σ min(background height, intensity) ✓. Area = total − background; Height = maximum − background ✓ | none inside one chromatogram: one peak per precursor and replicate | **clipped by construction**: a point below the background contributes its own intensity to the background, so 0 to Area ✓ (inferred from the formula) | Synchronized Integration: a manual change applies to the selected replicates or annotation groups, aligned to predicted RT or to the file's RT alignment; the nearest peak is used, so times can differ ✓. Import of boundaries from a text file ✓ | user-set peaks are kept on reintegration unless overwritten (u) |
| mzmine 4 [MZ] | local minimum resolver: splits a feature at chromatographic minima. "Min ratio of peak top/edge" 1.7–2 recommended ✓. Manual: Integration dashboard (older "Define manually" dialog) ✓ | not stated on the pages read | shoulders split into separate features at the minimum ✓ | (u) | join aligner (m/z and RT tolerance score) ✓. Gap filling re-integrates within m/z ± tolerance and RT ± tolerance of the row average ✓ | feature state: "Black – defined manually"; gap-filled = "ESTIMATED", grey ✓ |
| xcms [XM] | centWave: `integrate = 1` descends on the Mexican-hat filtered data, `2` on the real data ✓. matchedFilter: Gaussian filter with `fwhm`, `sigma = fwhm/2.3548` ✓ | `into` = raw integral; `intb` = "per-peak baseline corrected" integral (centWave only) (snippet) | overlapping peaks within `mzdiff` reduced to the largest ✓ | `intb` can become negative if the baseline estimate exceeds the signal (u, inference) | `adjustRtime` (peakGroups loess, obiwarp); `fillChromPeaks` integrates the feature's RT and m/z range on adjusted RTs; filled peaks `is_filled` ✓ | no manual edits in the standard workflow; `manualChromPeaks` (u) |
| OpenChrom [OC] | peak detectors plus manual peak by start and end (u) | baseline detectors (u) | (u) | (u) | (u) | (u) |

## 2. Notes by package
- **Thermo Xcalibur** [XC]:
  - The three detectors share one structure: smoothing points, an S/N threshold, and a "constrain peak width" switch with Peak Ht (%) and a tailing factor that stop the integration on long tails.
  - The baseline is the local minimum found within a window of scans ("Baseline window", ICIS). The peak edge comes from noise × "Area noise factor"; the peak threshold from noise × "Peak noise factor".
  - Noise is INCOS (one pass) or Repetitive (several passes) for ICIS. "Report noise as" is RMS or peak-to-peak, with peak-to-peak the default. RMS is forced when the user picks a manual noise region.
  - Avalon is the event-driven, ChemStation-like integrator: start/end threshold tied to RMS noise, bunch factor 1–6 (which "does not affect the final area"), area threshold, P-P resolution, negative peaks, tension, tangent skim.
- **Agilent ChemStation/OpenLab** [OL]: this is the reference vocabulary for event-based integration.
  - Peak-to-valley ratio = height of the smaller peak above the baseline / height of the valley above the baseline. Below the user value the pair is split by a drop line; above it each peak gets a valley baseline.
  - Skimming of a rider needs both criteria: skim height ratio Hp/Hc above the set value and skim valley ratio Hc/Hv below it.
  - Penetration of the baseline by the trace is a named condition. "Classical" accepts it; the other modes rebuild the baseline so the trace never dips below it.
  - Manual tools: new baseline, move baseline point, split peak (drop line), merge, delete, change baseline type. Corrections live with the injection, or are promoted to the method, which makes them reproducible on reprocessing.
- **MassHunter Quant** [MHQ]: the integrator is chosen per compound. Agile2 has no user parameters, so the reviewer only needs to audit peak filters and manual edits. "Apply to All" spreads parameter changes across compounds.
- **Waters** [WE][WKB]:
  - ApexTrack separates detection (apex, second derivative) from baseline placement (liftoff and touchdown slopes).
  - Its default for clusters is the perpendicular drop at the valley. Skims are explicit timed events with start and stop times placed "within the cluster of peaks".
  - TargetLynx review is a sample-by-sample slideshow that can pause for manual integration. Manual changes must be saved, or reprocessing drops them.
- **SCIEX** [SX]:
  - MQ4's "noise percentage" raises the baseline, which is the opposite of Federico's request because the user never sees where the line is. AutoPeak is the model-based successor.
  - The only setting verified here is noise percentage. The rest is listed so Federico can check it against his Analyst/MultiQuant installation.
- **Skyline** [SK]: the closest to what Federico asks for:
  - only two boundaries per peak, drawn as dashed lines;
  - a background that is a pure function of the two boundary intensities;
  - area defined so that nothing below the background is ever subtracted;
  - Synchronized Integration to repeat one decision across replicates.
  
  Its weakness: the flat background at the lower boundary intensity over-integrates a peak that sits on a sloping baseline. The support thread notes that areas are "very sensitive" to the window through the background term.
- **mzmine and xcms** [MZ][XM]: non-targeted. Boundaries come from the resolver, and gap-filled values are marked as estimates. These are the right models for mzLab's HR feature table, not for LR teaching.

## 3. Conventions and good practice
- **S/N** [PH]: Ph. Eur. 2.2.46 and USP <621> define S/N = 2H/h:
  - H is the peak height measured from the apex to the extrapolated baseline.
  - h is the peak-to-peak range of the background noise in a blank, measured over a window around the peak's position.
  
  The window was set to 20 × the width at half height in the 2022–2023 revisions. Ph. Eur. went back to 5 × from 1 January 2024. A Waters notice says USP also went back to 5 × from April 2023; no official USP text was read (u).
  
  ICH Q2(R2) uses S/N ≈ 3:1 for the detection limit and 10:1 for the quantitation limit (u: page not rendered).
  
  Vendors also offer RMS noise (Thermo, Avalon) and their own estimators (ICIS INCOS or Repetitive; MQ4 from the whole chromatogram; AutoPeak at the apex). The same peak can therefore have very different S/N values in different packages.
- **Peak-to-valley** [PH][OL]: Ph. Eur. allows p/v = Hp/Hv as a system-suitability criterion when two peaks are not baseline-separated (BP text, snippet). ChemStation uses the same ratio to choose a drop line or a valley baseline ✓.
- **Drop line vs skim** [LC] (snippets):
  - Dolan: when the minor peak is more than about 10 % of the major peak's height, a perpendicular drop to the baseline joining the true baseline before and after the group is best. Using the same method consistently matters more than the choice between a tangent and an exponential skim.
  - The LCGC "Integration errors" series found the drop method gave the least error in most of its tests. Peak height was often more accurate than area. For a small peak after a large one, skim plus height did better when the widths differ.
  - Better resolution remains the real fix. No single rule is settled, which is why the choice should be explicit and visible to the user.
- **Manual integration and data integrity** [FDA] ✓: the audit trail of an HPLC run "should include the user name, date/time of the run, the integration parameters used, and details of a reprocessing, if any", with "change justification for the reprocessing". Changes to chromatographic data "should be documented in an audit trail". mzLab is not a GMP system, but the same habit (who, what, when, why) is good for teaching and research.

## 4. Signal below the baseline inside the window
Federico's observation: with «linea personalizzata» the mzLab area is signed (above the line minus below, `int.base.intro`). When he drags the right edge past the point where the trace falls under the dashed line, the tail below the line is subtracted and the total shrinks. The packages differ:

| Package | Rule |
|---|---|
| Skyline | clipped: below-background points contribute 0 by the min() construction ✓ |
| ChemStation | "Classical" accepts the penetration. The other modes rebuild the baseline so that it does not cut the trace ✓. Whether "accepted" means a negative contribution is (u) |
| Thermo, Waters, SCIEX, MassHunter | not documented in the pages read (u). Their automatic baselines start and end on the trace, so penetration mostly arises after manual edits |
| xcms `intb` | signed in principle (u) |
| mzLab today | signed. The part below is reported separately (`below`, «Sotto la linea»), but subtracted from the area |

Reading: no package documents a signed area as the intended manual result. Where a rule is documented, it either clips the area or treats the penetration as a baseline error to repair. The proposal below follows the clip-and-show route.

## 5. Proposal for mzLab
### 5.1 One model: two anchors and the line through them
An integration is a record with two **anchors**, A (start) and B (end), on one trace. Each anchor has a time *t* and a **snap mode**:
- **curve**: y is the trace at *t*, linearly interpolated between points.
- **zero**: y = 0.
- **manual**: y is a free number set by dragging the point vertically or by typing it.

The baseline is the straight line through A and B. The area is measured between the trace and that line over [t_A, t_B]. The four current modes become combinations of the two anchors, so nothing is lost:

| Today | Anchor A | Anchor B |
|---|---|---|
| «da zero», «finestra fino a zero» | zero | zero |
| «linea tra i bordi» | curve | curve (today: mean of 3 points, see question 2) |
| «linea personalizzata» | manual | manual |
| new, e.g. a peak on the tail of the solvent front | curve | zero |

On the chart: two round points (24 × 24 px visual, 44 px touch target), the dashed baseline, and the fill between trace and line. There are **no vertical lines**. The window's extent is simply where the anchors are. Moving an anchor in time keeps its mode, so a curve anchor slides along the trace, as `intwin::move_end` already does. Dragging a curve or zero anchor vertically turns it into manual. A small menu on the point (right-click, long press, or the keys below) switches the mode.

### 5.2 Signal below the line
- **Default: clip.** Points under the baseline contribute 0. The clipped part is drawn hatched, as now under the dashed line, so it stays visible.
- The table shows it as its own column (**«Sotto la linea»**, already present), with a note when it is more than a few per cent of the area. The note means "the baseline probably cuts the peak", which is a teaching point in itself.
- The **signed** value (current behaviour) stays available per integration as an option (`below = clip | signed`). Old session records keep their stored numbers.
- This is COO14's inclination and matches the Skyline rule. It is question 1 for Federico.

### 5.3 Overlapping peaks
- **Drop line** (perpendicular from the trace to the baseline at a chosen time) is an explicit user action on an existing integration, never automatic in the low-resolution world. Commands: "Split here" at the cursor; "Split at the valley" moves the split to the local minimum between the two maxima, as a suggestion the user accepts.
- The two parts are two integrations that share the baseline and the split time. Their areas add up to the parent's. The table marks them `split`.
- **Tangent skim** needs no extra machinery: a rider integrated with both anchors on *curve* (valley and tail) *is* a straight tangent skim. To avoid counting the rider twice, the parent must carry a "minus rider" rule. Version 1 offers drop lines only and documents the skim as a second integration whose area overlaps the parent's (flagged `overlaps`). Exponential and Gaussian skims are not proposed: they are model choices that the evidence in §3 does not favour.
- In the table, the p/v ratio (Hp/Hv, Ph. Eur.) is shown next to a split. The student sees how resolved the pair is without the program choosing for them.

### 5.4 Propagating an integration to other samples
"Apply to other samples…" on an integration opens a list of the open files that have the same transition or XIC (m/z window and polarity). The user ticks the targets and chooses how time is carried:
- **Same times**: copy t_A and t_B.
- **Shift by apex**: move both anchors by the difference between the apex RT in the target and in the source, searched within ± the window width. This is the same idea as Skyline's alignment and TargetLynx's expected RT.

Each anchor is re-snapped in the target by its mode: curve takes the target's intensity, zero stays 0, manual keeps its *y* only if the user ticks "keep manual heights". Every copy is marked `propagated` (source file and integration) and `unconfirmed`. It looks different in the chart (dashed fill) and in the table until the user reviews it: Enter, or a click on «Conferma». A moved anchor turns the record into a normal manual one. The rest of the table never changes silently, and there is no "reintegrate all" that overwrites a confirmed record.

### 5.5 What is shown
- **Table** (one row per integration): trace, A (t, mode), B (t, mode), area, «Sotto la linea», height above the line, RT of the apex, S/N, width at half height, origin (`manual`, `auto` with parameters, `propagated`, `split`) and confirmation.
- **Baseline mode** is a column (e.g. "curve – zero"), not a label on the chart. The chart keeps only the points, the line and the fill, plus an optional small area label (existing behaviour).
- **S/N** follows Ph. Eur. 2H/h, with H above the integration's baseline and h the peak-to-peak noise over 5 × w½ in a region the user chooses (a blank or a quiet stretch). The noise region is shown and stored with the record. RMS is offered as an alternative, named in the column header.
- Copy-on-click of a value keeps working (INTEGRA-KEEP). Every chart with integrations keeps its `TDATI` table, with the anchors as rows.

### 5.6 Keyboard, touch, accessibility
- **Points**:
  - Each point is a `role=slider` with an `aria-label` such as "Start, follows the curve, 4.21 min".
  - Tab moves between points. ← → move one data point; Shift moves five.
  - ↑ ↓ change *y* by 1 % of the axis (Shift 10 %) and turn the point manual.
  - C, Z and M set the mode to curve, zero or manual. The keys must be checked against the existing B (baseline dialog) and the global shortcuts before they are chosen.
  - S splits at the cursor, V at the valley, Delete removes the integration. Ctrl+Z undoes one gesture.
- **Touch and pen**: drag the point; long press opens the mode menu; targets 44 px; `touch-action:none` only on the points.
- **Live region and theme**: a `role=status` line repeats point, mode and area after each step, as `.iep-live` does now. Colours come from CSS variables, and the fill and hatch differ by pattern as well as colour (Okabe-Ito). `prefers-reduced-motion` turns off any animation of the fill.
- **Tour**: `tour.js` gets one step on the points and their modes. `e2e_tour.py` stays valid.

### 5.7 Teaching principle and reproducibility
- **Low resolution**: the program never decides the peak or the baseline on its own. An integration exists only after a user gesture: drawing the two points, or the existing one-click tool, which proposes anchors the student can see and move. Its result is marked `auto` with its parameters. There is no automatic splitting, skimming or reintegration. Propagation is offered, never applied without a tick and a confirmation.
- **High resolution and non-targeted work**: automatic integration of all peaks (`auto_integrate`) stays. Its parameters (smoothing, S/N, AsLS λ) are shown in the table header and stored with the result. Its anchors are ordinary anchors (curve, curve), so the user can still edit any one peak.
- **Reproducibility**: an integration is fully described by trace identity, t_A, t_B, the two modes (and *y* for manual), the below rule, splits, the noise region and the origin. That record goes:
  - into the session;
  - into the **method**, as "integrations to repeat" per transition, with times and modes and no areas;
  - into the **report**, with areas and the engine version.
  
  Re-running the method on the same file gives the same bytes, in the site, the app and the CLI.

### 5.8 Implementation steps (each one PR; logic in Rust, parity site + app + CLI)
1. **Rust `intanchor` module** (`mzlab-core`):
   - resolve an anchor (curve / zero / manual → y);
   - area between the trace and the line through A and B, with `below = clip | signed`, plus `below` area, height, apex RT, w½;
   - S/N 2H/h with a noise region.
   
   It reuses `baseline::peak_area` for the signed case. Delivery: wasm export `int_anchor`, `peaks-worker.js` operation, CLI `mzlab-cli peak-integrate TRACE.csv --set a.t=… a.mode=curve b.t=… b.mode=zero below=clip`, a parity test of page bytes against CLI bytes, and analytic tests (Gaussian on a slope; a tail crossing the line). Record the wasm size before and after in `wasm-tetti.md`.
2. **Record migration** (JS reads, Rust converts): map `bm` = zero / edges / line and `zero:true` to anchor pairs. Old records keep their stored mode and numbers. The new `int/2` record (contract name to be fixed in `docs/agenti/`) is written only by new integrations. Delivery: a converter test.
3. **Chart and points** (JS, after INTEGRA-KEEP has removed the bars): draw the anchors, the mode menu and the keys of §5.6; hatch the clipped part; add the baseline-mode and origin columns. Update `e2e_integra_estremi`, `e2e_integra_modi` and the tour.
4. **Splits** (Rust `intanchor::split`, the valley search, p/v; CLI `--set split=MIN`), then the JS commands S and V. Delivery: e2e with mouse, keyboard and touch.
5. **Propagation** (Rust `intanchor::propagate`: source record and target trace in, anchors and shift out; CLI `peak-propagate`), then the JS dialog, the `propagated` / `unconfirmed` flags and the confirm action.
6. **Method and report**: store the integrations in the method, repeat them on load (user-triggered), and write them into the report, with a parity test of method → CLI → same JSON.
7. **Docs**: rewrite the integration lines of `dati-integrazione.md` and `transizioni.md` for the anchor model; one i18n key set in `lang/it.js` and `lang/en.js`.

## 6. Open questions for Federico
1. When the trace dips under the line inside the window: **clip** and show the clipped part hatched (proposed default), or keep the **signed** area (today)? Should the choice be per integration or one global setting?
2. "Follow the curve": the raw point at that time (interpolated), the mean of 3 points as «linea tra i bordi» does today, or a lightly smoothed value? On noisy SRM traces the choice changes the baseline height.
3. Propagation to other samples: same absolute times, or shifted by the apex RT difference? When an internal standard is present, should its RT drive the shift?
4. Overlaps: are drop lines enough for version 1, or do you need tangent skim (rider on a tail) with the parent's area reduced accordingly?
5. S/N: is Ph. Eur. 2H/h over 5 × w½ in a chosen blank region the right default for your reports, or do you need RMS noise (as in Thermo or Analyst) to compare with numbers from the instrument PC?

## 7. Not verified
These rest on general knowledge or snippets only:
- the MQ4 / IntelliQuan parameter definitions beyond noise percentage, and SCIEX manual-integration flags;
- Xcalibur Quan Browser manual-integration gestures and flags;
- MassLynx legacy integrator options and TargetLynx flag codes;
- ChemStation timed events, baseline codes and the meaning of "accepts penetrations" for the area;
- MassHunter manual-integration gestures and RT-window update;
- Skyline "Apply Peak to All" and reintegration options;
- mzmine area baseline handling;
- negative `intb` in xcms;
- all of OpenChrom;
- the current USP S/N window and the ICH Q2(R2) wording.

Each is marked (u) where it appears. Before implementation, check them against the manuals installed on the instrument PCs.
