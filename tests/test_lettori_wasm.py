"""The readers modules (crates/mzlab-lettori-wasm, one .wasm per family) stay modules of their own: their size limits do not go up, the engine (mzlab-wasm) does not name them,
every family has its feature, and a module that is built stays under its limit."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TETTI = {"aperti": 561152, "andi": 344064, "thermo": 774144, "agilent": 606208, "sciex": 475136, "waters": 380928, "shimadzu": 380928, "bruker": 532480, "valida": 479744}


def tabella():
    s = (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    blocco = re.search(r'^tetti="(.*?)"', s, re.M | re.S).group(1)
    return {f: int(n) for f, n in (r.split() for r in blocco.strip().splitlines())}


def test_tetti_nello_script():
    t = tabella()
    assert set(t) == set(TETTI)
    for f, n in t.items():
        assert n <= TETTI[f], (f, n)


def test_il_motore_non_nomina_i_moduli():
    t = (ROOT / "crates/mzlab-wasm/Cargo.toml").read_text(encoding="utf-8")
    assert "mzlab-lettori" not in t and "estesi" not in t


def test_ogni_famiglia_ha_la_sua_feature():
    for crate in ("mzlab-lettori", "mzlab-lettori-wasm"):
        c = (ROOT / "crates" / crate / "Cargo.toml").read_text(encoding="utf-8")
        for f in TETTI:
            if f == "valida" and crate == "mzlab-lettori":    # the validator reads no maker's format
                continue
            assert re.search(rf"^{f} = ", c, re.M), (crate, f)


def test_costruiti_dentro_il_tetto():
    for f, n in TETTI.items():
        w = ROOT / "site/static/wasm-lettori" / f / "mzlab_lettori_wasm_bg.wasm"
        if w.exists():
            assert w.stat().st_size < n, (f, w.stat().st_size)


def test_costruzione_chiamata_da_build_feat():
    assert "tools/build_lettori_wasm.sh" in (ROOT / "tools/build_feat_wasm.sh").read_text(encoding="utf-8")


def test_passi_estesi_una_sola_lista():
    """The extra wasm-opt passes live in tools/wasm_opt.sh only; the readers and the engine/libraries steps of the four workflows name them as `@motore` (the readers use `@estesi`)."""
    sh = (ROOT / "tools/wasm_opt.sh").read_text(encoding="utf-8")
    assert re.search(r'^estesi="--converge .*--low-memory-unused', sh, re.M)
    assert "@estesi" in (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    assert "--converge" not in (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    for wf in ("pages", "sito-rust", "release", "notte"):
        righe = [r for r in (ROOT / f".github/workflows/{wf}.yml").read_text(encoding="utf-8").splitlines() if "tools/wasm_opt.sh site/static" in r]
        assert len(righe) == 2 and all(r.rstrip().endswith("@motore") for r in righe), (wf, righe)


def test_wasm_opt_espande_estesi(tmp_path):
    """`@estesi` becomes the passes (and is never passed on as such) to the wasm-opt it runs; checked with a stand-in wasm-opt that writes its arguments."""
    import shutil
    import subprocess

    if not shutil.which("flock") or not shutil.which("bash"):
        return
    rt = tmp_path / "rt"
    (rt / "binaryen-133/bin").mkdir(parents=True)
    fake = rt / "binaryen-133/bin/wasm-opt"
    fake.write_text(f'#!/usr/bin/env bash\necho "$@" > {tmp_path}/args\n')
    fake.chmod(0o755)
    (rt / "binaryen-133/installato").write_text("")
    wasm = tmp_path / "m_bg.wasm"
    wasm.write_bytes(b"\0asm\1\0\0\0")
    env = {"PATH": "/usr/bin:/bin:/usr/local/bin", "RUNNER_TEMP": str(rt)}
    r = subprocess.run(["bash", str(ROOT / "tools/wasm_opt.sh"), str(wasm), "1000", "t", "@estesi"], env=env, capture_output=True, text=True, cwd=ROOT)
    assert r.returncode == 0, r.stdout + r.stderr
    args = (tmp_path / "args").read_text().split()
    assert "@estesi" not in args and "--low-memory-unused" in args and "--converge" in args and args[0] == "-O3"
    r = subprocess.run(["bash", str(ROOT / "tools/wasm_opt.sh"), str(wasm), "1000", "t", "@motore"], env=env, capture_output=True, text=True, cwd=ROOT)
    args = (tmp_path / "args").read_text().split()
    assert r.returncode == 0 and "@motore" not in args and "--reorder-functions" in args and "--const-hoisting" not in args and "--converge" not in args
    r = subprocess.run(["bash", str(ROOT / "tools/wasm_opt.sh"), str(wasm), "1000", "t"], env=env, capture_output=True, text=True, cwd=ROOT)
    assert r.returncode == 0 and "--converge" not in (tmp_path / "args").read_text()


def _dati_minimi(b: bytes):
    """The lowest address of an active data segment of a .wasm (None without any)."""
    def leb(i):
        r = s = 0
        while True:
            x = b[i]
            i += 1
            r |= (x & 0x7F) << s
            s += 7
            if not x & 0x80:
                return r, i

    i, minimo = 8, None
    while i < len(b):
        sid = b[i]
        sz, j = leb(i + 1)
        if sid == 11:
            n, k = leb(j)
            for _ in range(n):
                flag, k = leb(k)
                if flag != 0 or b[k] != 0x41:
                    return minimo  # a passive or multi-memory segment: not the layout of rustc
                off, k = leb(k + 1)
                minimo = off if minimo is None else min(minimo, off)
                k += 1
                ln, k = leb(k)
                k += ln
        i = j + sz
    return minimo


def test_la_memoria_sotto_1024_non_e_dei_dati():
    """`--low-memory-unused` is safe because rustc links the stack first: no data lives below 1024 (the .wasm built here or by the CI, when present)."""
    for w in [*(ROOT / "target/wasm32-unknown-unknown/release").glob("mzlab_*.wasm"), *(ROOT / "site/static").glob("wasm*/**/*_bg.wasm")]:
        m = _dati_minimi(w.read_bytes())
        assert m is None or m >= 1024, (w.name, m)
