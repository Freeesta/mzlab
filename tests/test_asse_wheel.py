"""Wheel / pinch zoom factor shared by the charts (ASSE.wheelFactor): trackpad sensitivity, notch unchanged, per-event clamp."""
import json, shutil, subprocess, pathlib, pytest

@pytest.mark.skipif(not shutil.which("node"), reason="node missing")
def test_wheel_factor():
    out = subprocess.run(["node", str(pathlib.Path(__file__).with_name("asse_wheel_node.js"))], capture_output=True, text=True, check=True).stdout
    r = json.loads(out)
    assert all(v is True for v in r.values()), r
