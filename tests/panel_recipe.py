"""The recipe text the Pipeline panel writes for a unit-resolution file with the `nominal` profile switched on and every other field at its default.
One copy for the byte test (tests/test_feat_nominal_wasm.py) and the browser test (tests_e2e/e2e_pipeline_nominal.py): the profile writes `floor` and
`min_height` itself, so the panel leaves them out while they hold the profile's values."""
import json

PANEL_NOMINAL = {"polarity": 1, "mode": "family", "ppm": 5, "min_points": 5, "rt_tol": 0.1, "fill": True, "preset": "nominal"}
PANEL_NOMINAL_TEXT = json.dumps(PANEL_NOMINAL, separators=(",", ":"))
