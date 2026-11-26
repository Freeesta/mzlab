Read docs/coda/BRIEF.md.

# CLICK-LABEL-HOVER: hover label and keyboard all-channels action

**Goal.** Labels never overlap and the selected integration is always labelled (`area-etichette.js`); an unselected one is left out when no row is free. Add the hovered integration (pointer over its window or its table row), and, if the owner decides overlaid panels should label only the selected or hovered integration, make that so (`e2e_integrazione_auto`, `e2e_integrazione_multi` and `e2e_sim_wiff` now assert a label per integration and would change). Add a keyboard action "integrate all channels at this time" (`INTCLICK.atRt`, never forced) with a binding in `azioni-tasti.js`.

**Why.** Four labels at one time were unreadable; they are stacked now, but a crowded overlay still drops labels. The all-channel action is reachable only from the menu.

**Read first.** `docs/agenti/data-integration.md` ("Area labels", "Click guard"), `mzlab/web/area-etichette.js`, `mzlab/web/integrazioni-manuali.js` (the table rows), `mzlab/web/azioni-tasti.js`, `mzlab/web/integra-clic.js`.

**Done means.** A hover state of an integration (`p.ihov`) set by the plot pointer and the table row, drawn by `AREALBL` after the selected one, non-overlapping and in the trace colour; the shortcut listed in the keyboard help with a `lang/` key; e2e with the synthetic four-ion file (`sim_overlay_file`); axe and keyboard rules respected (`prefers-reduced-motion`, 24 px targets).

**Out of scope.** The engine and its guard.

**Follow-ups.** Closing step of the brief.
