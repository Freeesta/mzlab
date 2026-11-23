"""tools/install.sh offline: a local HTTP server stands in for releases/latest/download, a fake `uname` sets the platform.
Every run has a PATH of only the tools the script may use (one downloader at a time), so a hidden dependency fails here.
Checks the asset choice, the SHA256SUMS check, the distinct exit codes and that nothing is installed on a failure."""
from __future__ import annotations

import hashlib
import http.server
import os
import shutil
import subprocess
import sys
import threading
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "tools" / "install.sh"
sys.path.insert(0, str(ROOT / "tools"))
import note_release  # noqa: E402

CLI = [note_release.nomi("1.0.0")[k] for k in ("cli_x86_64", "cli_aarch64", "cli_mac")]
TOOLS = ["sh", "awk", "chmod", "cp", "head", "mkdir", "mktemp", "mv", "rm", "sha256sum", "tr", "cat"]
SHELL = shutil.which("dash") or shutil.which("sh")


def tool(name: str) -> str:
    """The system copy first: a wrapper earlier in the developer's PATH may refuse to run outside its own shell."""
    return shutil.which(name, path="/usr/bin:/bin:/usr/local/bin") or shutil.which(name)


def fake_binary(name: str) -> bytes:
    return f"#!/bin/sh\necho 'mzlab-cli 9.9.9 ({name})'\n".encode()


class Server:
    """Serves `files` (name -> bytes) and records the requested paths."""

    def __init__(self):
        self.files: dict[str, bytes] = {}
        self.requests: list[str] = []
        outer = self

        class H(http.server.BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802
                outer.requests.append(self.path)
                body = outer.files.get(self.path.lstrip("/"))
                self.send_response(200 if body is not None else 404)
                self.send_header("Content-Length", str(len(body or b"")))
                self.end_headers()
                self.wfile.write(body or b"")

            def log_message(self, *a):
                pass

        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
        self.url = f"http://127.0.0.1:{self.httpd.server_address[1]}/latest/download"
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def release(self, sums: dict[str, str] | None = None, drop: tuple[str, ...] = ()):
        self.files = {f"latest/download/{n}": fake_binary(n) for n in CLI if n not in drop}
        lines = sums if sums is not None else {n: hashlib.sha256(fake_binary(n)).hexdigest() for n in CLI}
        self.files["latest/download/SHA256SUMS"] = "".join(f"{h}  {n}\n" for n, h in lines.items()).encode()


@pytest.fixture(scope="module")
def server():
    s = Server()
    yield s
    s.httpd.shutdown()


def sandbox(tmp: Path, system: str, machine: str, downloader: str | None, sysctl: str | None = None) -> Path:
    b = tmp / "bin"
    b.mkdir()
    for t in TOOLS + ([downloader] if downloader else []):
        os.symlink(tool(t), b / t)
    (b / "uname").write_text(f'#!{SHELL}\ncase "$1" in -s) echo {system};; -m) echo {machine};; esac\n')
    (b / "uname").chmod(0o755)
    if sysctl is not None:
        (b / "sysctl").write_text(f"#!{SHELL}\necho {sysctl}\n")
        (b / "sysctl").chmod(0o755)
    return b


def run(tmp: Path, server: Server, *args: str, system="Linux", machine="x86_64", downloader="curl", sysctl=None, env=None):
    b = sandbox(tmp, system, machine, downloader, sysctl)
    e = {"PATH": str(b), "HOME": str(tmp / "home"), "MZLAB_BASE_URL": server.url, "NO_PROXY": "*", "no_proxy": "*"}
    e.update(env or {})
    server.requests.clear()
    return subprocess.run([SHELL, str(SCRIPT), *args], env=e, capture_output=True, text=True, timeout=60)


@pytest.mark.parametrize("downloader", ["curl", "wget"])
@pytest.mark.parametrize(
    "system,machine,sysctl,asset",
    [
        ("Linux", "x86_64", None, "mzlab-cli-linux-x86_64"),
        ("Linux", "aarch64", None, "mzlab-cli-linux-aarch64"),
        ("Darwin", "arm64", None, "mzlab-cli-macos-aarch64"),
        ("Darwin", "x86_64", "1", "mzlab-cli-macos-aarch64"),  # Rosetta shell on Apple silicon
    ],
)
def test_installs_the_verified_binary_of_the_platform(tmp_path, server, downloader, system, machine, sysctl, asset):
    if not tool(downloader):
        pytest.skip(f"{downloader} is not installed on this machine")
    assert asset in CLI
    server.release()
    r = run(tmp_path, server, system=system, machine=machine, downloader=downloader, sysctl=sysctl)
    assert r.returncode == 0, r.stderr
    dest = tmp_path / "home" / ".local" / "bin" / "mzlab-cli"
    assert dest.read_bytes() == fake_binary(asset) and os.access(dest, os.X_OK)
    assert r.stdout.splitlines() == [str(dest), f"mzlab-cli 9.9.9 ({asset})"]
    assert "matches SHA256SUMS" in r.stderr and "not in PATH" in r.stderr
    assert server.requests == [f"/latest/download/{asset}", "/latest/download/SHA256SUMS"]
    assert [p.name for p in dest.parent.iterdir()] == ["mzlab-cli"]


@pytest.mark.parametrize("system,machine", [("Linux", "riscv64"), ("Darwin", "x86_64"), ("MINGW64_NT-10.0", "x86_64")])
def test_unsupported_platform_exits_3_without_any_request(tmp_path, server, system, machine):
    server.release()
    r = run(tmp_path, server, system=system, machine=machine, sysctl="0" if system == "Darwin" else None)
    assert r.returncode == 3, r.stderr
    assert "no mzlab-cli binary for" in r.stderr and "cargo build --profile cli -p mzlab-cli" in r.stderr
    assert server.requests == []


def test_release_without_the_asset_exits_5(tmp_path, server):
    server.release(drop=("mzlab-cli-linux-x86_64",))
    r = run(tmp_path, server)
    assert r.returncode == 5, r.stderr
    assert "the release has no asset mzlab-cli-linux-x86_64" in r.stderr and "HTTP 404" in r.stderr
    assert not (tmp_path / "home" / ".local").exists()


def test_checksum_differs_exits_7_and_keeps_the_installed_binary(tmp_path, server):
    server.release(sums={n: "0" * 64 for n in CLI})
    old = tmp_path / "home" / ".local" / "bin" / "mzlab-cli"
    old.parent.mkdir(parents=True)
    old.write_bytes(b"old")
    r = run(tmp_path, server)
    assert r.returncode == 7, r.stderr
    assert "checksum differs for mzlab-cli-linux-x86_64" in r.stderr and "nothing installed" in r.stderr
    assert old.read_bytes() == b"old" and [p.name for p in old.parent.iterdir()] == ["mzlab-cli"]


def test_asset_missing_from_sha256sums_exits_7(tmp_path, server):
    server.release(sums={"mzlab-cli-linux-aarch64": "0" * 64})
    r = run(tmp_path, server)
    assert r.returncode == 7 and "SHA256SUMS does not list mzlab-cli-linux-x86_64" in r.stderr


def test_release_without_sha256sums_exits_7(tmp_path, server):
    server.release()
    del server.files["latest/download/SHA256SUMS"]
    r = run(tmp_path, server)
    assert r.returncode == 7 and "the release has no SHA256SUMS" in r.stderr
    assert not (tmp_path / "home" / ".local").exists()


def test_no_server_exits_6(tmp_path, server):
    server.release()
    r = run(tmp_path, server, env={"MZLAB_BASE_URL": "http://127.0.0.1:9/latest/download"})
    assert r.returncode == 6 and "failed (no response)" in r.stderr


def test_no_downloader_exits_4(tmp_path, server):
    server.release()
    r = run(tmp_path, server, downloader=None)
    assert r.returncode == 4 and "neither curl nor wget" in r.stderr


def test_dry_run_downloads_nothing_and_names_the_urls(tmp_path, server):
    server.release()
    r = run(tmp_path, server, "--dry-run", "--version", "v1.2.3", "--dir", str(tmp_path / "d"), env={"MZLAB_BASE_URL": ""})
    assert r.returncode == 0, r.stderr
    kv = dict(line.split("=", 1) for line in r.stdout.splitlines())
    assert kv["url"] == "https://github.com/Freeesta/mzlab/releases/download/v1.2.3/mzlab-cli-linux-x86_64"
    assert kv["sums"].endswith("/v1.2.3/SHA256SUMS") and kv["target"] == str(tmp_path / "d" / "mzlab-cli")
    assert server.requests == [] and not (tmp_path / "d").exists()


def test_default_url_is_the_latest_release(tmp_path, server):
    r = run(tmp_path, server, "--dry-run", env={"MZLAB_BASE_URL": ""})
    kv = dict(line.split("=", 1) for line in r.stdout.splitlines())
    assert kv["url"] == "https://github.com/Freeesta/mzlab/releases/latest/download/mzlab-cli-linux-x86_64"
    assert kv["target"] == str(tmp_path / "home" / ".local" / "bin" / "mzlab-cli")


def test_wrong_usage_exits_2(tmp_path, server):
    r = run(tmp_path, server, "--bogus")
    assert r.returncode == 2 and "unknown argument: --bogus" in r.stderr


def test_release_workflow_attaches_the_names_the_script_fetches():
    yml = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
    for n in CLI:
        assert f"' {n}\n" in yml, n
