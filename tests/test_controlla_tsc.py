"""tools/tipi/controlla_tsc.py: the npx install of tsc is killed and retried when it hangs."""
import importlib.util
import os
import sys
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("controlla_tsc", ROOT / "tools" / "tipi" / "controlla_tsc.py")
ct = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ct)


@pytest.mark.skipif(not hasattr(os, "killpg"), reason="process groups are POSIX only")
def test_hung_install_is_killed_with_its_children_and_gives_a_clear_error(tmp_path, capsys):
    marca = tmp_path / "figlio.pid"
    # the parent starts a child (like npx does) and both sleep forever
    codice = f"import subprocess,sys,time; c=subprocess.Popen([sys.executable,'-c','import time; time.sleep(600)']); open({str(marca)!r},'a').write(str(c.pid)+'\\n'); time.sleep(600)"
    t = time.time()
    with pytest.raises(SystemExit) as e:
        ct.installa([sys.executable, "-c", codice], timeout=1, prove=2)
    assert time.time() - t < 20 and "could not be installed" in str(e.value) and "2 tries" in str(e.value)
    assert capsys.readouterr().out.count("hung for 1 s, killed") == 2
    for pid in marca.read_text().split():
        with pytest.raises(ProcessLookupError):
            for _ in range(50):           # the kill is asynchronous: wait for the reaping
                os.kill(int(pid), 0)
                time.sleep(0.1)


def test_install_returns_at_the_first_good_try():
    ct.installa([sys.executable, "-c", "pass"], timeout=5, prove=3)


def test_a_stretto2_error_fails_even_when_the_base_is_silent(monkeypatch, tmp_path, capsys):
    base = tmp_path / "base.json"
    base.write_text("{}")
    monkeypatch.setattr(ct, "BASE", str(base))
    monkeypatch.setattr(ct, "misura", lambda: {"stretto2:mzlab/web/x.js": 1})
    monkeypatch.setattr(sys, "argv", ["controlla_tsc.py"])
    assert ct.main() == 1 and "stretto2:mzlab/web/x.js: 1 errors" in capsys.readouterr().out


def test_stretto_keys_are_ignored_in_the_base(monkeypatch, tmp_path):
    base = tmp_path / "base.json"
    base.write_text('{"stretto2:mzlab/web/x.js": 5}')
    monkeypatch.setattr(ct, "BASE", str(base))
    monkeypatch.setattr(ct, "misura", lambda: {"stretto2:mzlab/web/x.js": 2})
    monkeypatch.setattr(sys, "argv", ["controlla_tsc.py"])
    assert ct.main() == 1
