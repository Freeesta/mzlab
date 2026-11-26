Read docs/coda/BRIEF.md.

# CLICK-DRAG-ONE-TRACE: manual windows on one trace

**Goal.** Dragging a window (`imode` man) on a SIM/MRM panel integrates the trace under the pointer only; the same window on every channel stays available as an explicit menu action ("same window on all channels"), one undo step.

**Why.** The click of the automatic tool already integrates one trace (`integra-clic.js`: `targets`). The manual drag still goes through `intTargets` in `eventi.js`, and SIM/MRM panels are opened with `intf: "all", intfFixed: true`, so one drag makes one row per channel, mostly junk windows on channels without a peak.

**Read first.** `grep -n 'integration\|click' docs/agenti/INDICE.md`, then `docs/agenti/data-integration.md` ("Click guard"); `mzlab/web/eventi.js` (the `imode === "man"` branch), `mzlab/web/picchi.js` (`intTargets`, `intSeries`), `mzlab/web/integra-clic.js` (`nearest`, `atRt`).

**Done means.** A drag on an overlaid SIM panel gives one integration on the nearest trace (highlighted); the all-channel action exists in the menu with a `lang/` key; `tests_e2e/e2e_integra_clic.py` (or a new e2e) pins it with the synthetic four-ion file of `tools/synthetic_data_jagged.py` (`sim_overlay_file`); drag behaviour on free panels with "all traces" is unchanged. Keep the guided tour valid.

**Out of scope.** The engine guard of the click (`peakq_click.rs`); HR traces.

**Follow-ups.** Closing step of the brief.
