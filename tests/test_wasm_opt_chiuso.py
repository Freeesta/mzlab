"""tools/wasm_opt.sh fails closed in the production workflows: with MZLAB_WASM_OPT_STRICT=1 and no wasm-opt it is an error, without it a warning.
The binaryen tarball cached by actions/cache is reused when its SHA-256 is the pinned one; a wrong hash stays an error.
Every job of pages.yml, sito-rust.yml and release.yml that runs wasm-opt sets the strict variable and restores the tarball cache;
the nightly restores the same cache but stays lenient."""
import hashlib
import io
import os
import platform
import re
import shutil
import subprocess
import tarfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "tools/wasm_opt.sh"
CACHE_KEY = "binaryen-version_133-${{ runner.os }}"
WORKFLOWS = ("pages", "sito-rust", "release")

FAKE_CURL = '#!/usr/bin/env bash\necho called >> "{log}"\n{body}'
GARBAGE = 'while [ $# -gt 0 ]; do [ "$1" = -o ] && out=$2; shift; done; echo junk > "$out"\n'


WINDOWS = platform.system() == "Windows"


def _git_bash():
    """Git for Windows' bash.exe, next to git: a bare `bash` on a Windows runner is the WSL launcher in System32, which has no distribution."""
    git = shutil.which("git")
    # git is <Git>/cmd/git.exe, <Git>/bin/git.exe or <Git>/mingw64/bin/git.exe
    for root in Path(git).resolve().parents[1:3] if git else ():
        if (root / "bin" / "bash.exe").is_file() and (root / "usr" / "bin").is_dir():
            return root / "bin" / "bash.exe"
    pytest.fail(f"Git for Windows' bash.exe not found next to git ({git})")


def _bin(tmp_path, curl_body="exit 1\n"):
    b = tmp_path / "bin"
    b.mkdir()
    (b / "curl").write_text(FAKE_CURL.format(log=(tmp_path / "curl.log").as_posix(), body=curl_body))
    (b / "npm").write_text("#!/usr/bin/env bash\nexit 1\n")
    for f in b.iterdir():
        f.chmod(0o755)
    return b


def _run(tmp_path, script=SCRIPT, strict=True, curl_body="exit 1\n"):
    rt = tmp_path / "rt"
    rt.mkdir(exist_ok=True)
    wasm = tmp_path / "m.wasm"
    wasm.write_bytes(b"\0asm" + b"\0" * 10)
    fake = _bin(tmp_path, curl_body)
    if WINDOWS:
        # a Windows PATH (`;`), which bash converts at start-up; the stand-ins come first, then Git's own tools (uname, mkdir, cut, ...)
        bash = _git_bash()
        git = bash.parent.parent
        env = {"PATH": os.pathsep.join([str(fake), str(git / "usr" / "bin"), str(git / "bin")]), "RUNNER_TEMP": rt.as_posix(),
               "SYSTEMROOT": os.environ.get("SYSTEMROOT", r"C:\Windows")}   # Windows programs started without it fail to load their DLLs
    else:
        bash = "bash"
        env = {"PATH": f"{fake}:/usr/bin:/bin:/usr/local/bin", "RUNNER_TEMP": str(rt)}
    if strict:
        env["MZLAB_WASM_OPT_STRICT"] = "1"
    return subprocess.run([str(bash), str(script), str(wasm), "1000", "t"], env=env, capture_output=True, text=True, cwd=ROOT)


def test_strict_without_wasm_opt_is_an_error(tmp_path):
    r = _run(tmp_path)
    assert r.returncode == 1, r.stdout + r.stderr
    assert "::error::wasm-opt could not be installed" in r.stdout


def test_local_without_wasm_opt_is_a_warning(tmp_path):
    r = _run(tmp_path, strict=False)
    assert r.returncode == 0, r.stdout + r.stderr
    assert "::warning::wasm-opt could not be installed" in r.stdout


@pytest.mark.skipif(f"{platform.system()}-{platform.machine()}" != "Linux-x86_64", reason="the fake tarball has the Linux layout")
def test_sha_mismatch_stays_an_error(tmp_path):
    r = _run(tmp_path, strict=False, curl_body=GARBAGE)
    assert r.returncode == 1, r.stdout + r.stderr
    assert "SHA-256 mismatch" in r.stdout


@pytest.mark.skipif(f"{platform.system()}-{platform.machine()}" != "Linux-x86_64", reason="the fake tarball has the Linux layout")
def test_cached_tarball_is_used_without_download(tmp_path):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        body = f'#!/usr/bin/env bash\necho "$@" > {tmp_path}/args\n'.encode()
        info = tarfile.TarInfo("binaryen-version_133/bin/wasm-opt")
        info.size, info.mode = len(body), 0o755
        t.addfile(info, io.BytesIO(body))
    rt = tmp_path / "rt"
    rt.mkdir()
    (rt / "binaryen-133.tar.gz").write_bytes(buf.getvalue())
    # a copy of the script pinned on the fake tarball: the real pin cannot match a test file
    sha = hashlib.sha256(buf.getvalue()).hexdigest()
    src = SCRIPT.read_text(encoding="utf-8")
    pinned = re.search(r"x86_64-linux\.tar\.gz;\s+sha=([0-9a-f]{64})", src).group(1)
    copy = tmp_path / "wasm_opt.sh"
    copy.write_text(src.replace(pinned, sha))
    r = _run(tmp_path, script=copy)
    assert r.returncode == 0, r.stdout + r.stderr
    assert "binaryen 133: cached tarball" in r.stdout
    assert not (tmp_path / "curl.log").exists()
    assert (tmp_path / "args").read_text().startswith("-O3 ")


def _env_and_jobs(wf):
    """The top-level env lines and {job: its text}, read as text: the CI Python has no YAML parser."""
    text = (ROOT / f".github/workflows/{wf}.yml").read_text(encoding="utf-8")
    head, body = re.split(r"^jobs:\n", text, maxsplit=1, flags=re.M)
    parts = re.split(r"^  ([A-Za-z0-9_-]+):\n", body, flags=re.M)
    return head, dict(zip(parts[1::2], parts[2::2]))


def _builds_wasm(job):
    return "tools/wasm_opt.sh" in job or re.search(r"build_feat_wasm\.sh site/static( [ab])?\s*$", job, re.M)


CACHE_STEP = re.compile(r"- uses: actions/cache@[0-9a-f]{40} # v[\d.]+\n\s+with:\n\s+path: \$\{\{ runner\.temp \}\}/binaryen-133\.tar\.gz\n\s+key: " + re.escape(CACHE_KEY) + r"\n")


@pytest.mark.parametrize("wf", WORKFLOWS)
def test_production_workflows_fail_closed_and_cache_binaryen(wf):
    head, jobs = _env_and_jobs(wf)
    assert re.search(r'^env:\n(?:  .*\n)*  MZLAB_WASM_OPT_STRICT: "1"$', head, re.M), wf
    wasm = {n: j for n, j in jobs.items() if _builds_wasm(j)}
    assert wasm, wf
    for name, job in wasm.items():
        assert len(CACHE_STEP.findall(job)) == 1, (wf, name)


def test_the_night_stays_lenient_and_caches_binaryen():
    assert "MZLAB_WASM_OPT_STRICT" not in (ROOT / ".github/workflows/notte.yml").read_text(encoding="utf-8")
    _, jobs = _env_and_jobs("notte")
    wasm = {n: j for n, j in jobs.items() if "tools/wasm_opt.sh" in j or "tools/build_feat_wasm.sh site/static \"" in j}
    assert set(wasm) == {"sito"}, sorted(wasm)
    assert len(CACHE_STEP.findall(wasm["sito"])) == 1


def _gate(tmp_path, limit, wasm_size=100, stat_body=None, wc_body=None):
    """Run the size gate with a fake already-installed wasm-opt (a no-op) and, optionally, a `stat` stand-in (a PATH shim, like macOS where `stat -c` fails)."""
    rt = tmp_path / "rt"
    (rt / "binaryen-133" / "bin").mkdir(parents=True)
    wo = rt / "binaryen-133" / "bin" / "wasm-opt"
    wo.write_text("#!/usr/bin/env bash\nexit 0\n")   # in-place no-op: the file is left as it is
    wo.chmod(0o755)
    (rt / "binaryen-133" / "installato").write_text("")
    fake = tmp_path / "bin"
    fake.mkdir()
    if wc_body is not None:
        (fake / "wc").write_text("#!/usr/bin/env bash\n" + wc_body)
        (fake / "wc").chmod(0o755)
    if stat_body is not None:
        (fake / "stat").write_text("#!/usr/bin/env bash\n" + stat_body)
        (fake / "stat").chmod(0o755)
    wasm = tmp_path / "m.wasm"
    wasm.write_bytes(b"\0" * wasm_size)
    env = {"PATH": f"{fake}:/usr/bin:/bin", "RUNNER_TEMP": str(rt), "MZLAB_WASM_OPT_STRICT": "1"}
    return subprocess.run(["bash", str(SCRIPT), str(wasm), str(limit), "t"], env=env, capture_output=True, text=True, cwd=ROOT)


@pytest.mark.skipif(WINDOWS or platform.system() != "Linux", reason="uses the Linux binaryen install marker")
def test_size_gate_does_not_depend_on_gnu_stat(tmp_path):
    r = _gate(tmp_path, 1000, stat_body="echo 'stat: illegal option -- c' >&2\nexit 1\n")
    assert r.returncode == 0, r.stdout + r.stderr
    assert "t .wasm (optimised): 100 bytes" in r.stdout


@pytest.mark.skipif(WINDOWS or platform.system() != "Linux", reason="uses the Linux binaryen install marker")
def test_size_gate_fails_closed_on_unreadable_size(tmp_path):
    # a size that is not a number (a broken `wc`) must not pass the gate
    r = _gate(tmp_path, 1000, wc_body="echo oops\n")
    assert r.returncode == 1, r.stdout + r.stderr
    assert "::error::" in r.stdout


@pytest.mark.skipif(WINDOWS or platform.system() != "Linux", reason="uses the Linux binaryen install marker")
@pytest.mark.parametrize("limit,rc", [(101, 0), (100, 1), (99, 1)])
def test_size_gate_cap_is_exclusive(tmp_path, limit, rc):
    r = _gate(tmp_path, limit)
    assert r.returncode == rc, r.stdout + r.stderr
