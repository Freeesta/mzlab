"""tools/cli_prebuilt.py against a local server, and the shape of .github/workflows/cli-main.yml (a rolling pre-release that cannot pass for a release)."""
import hashlib
import http.server
import json
import re
import sys
import threading
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import cli_prebuilt as cp  # noqa: E402

SHA = "0123456789abcdef0123456789abcdef01234567"
NAME = f"mzlab-cli-{SHA[:12]}-linux-x86_64"


def fake(commit: str) -> bytes:
    return f"#!/bin/sh\necho '{{\"name\": \"mzlab-cli\", \"commit\": \"{commit}\"}}'\n".encode()


@pytest.fixture()
def server(monkeypatch):
    files, seen = {}, []

    class H(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            seen.append(self.path)
            body = files.get(self.path.lstrip("/"))
            self.send_response(200 if body is not None else 404)
            self.send_header("Content-Length", str(len(body or b"")))
            self.end_headers()
            self.wfile.write(body or b"")

        def log_message(self, *a):
            pass

    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    monkeypatch.setenv("MZLAB_BASE_URL", f"http://127.0.0.1:{httpd.server_address[1]}/rel")
    monkeypatch.setenv("NO_PROXY", "*")
    monkeypatch.setattr(cp.platform, "system", lambda: "Linux")
    monkeypatch.setattr(cp.platform, "machine", lambda: "x86_64")
    files["rel/LATEST"] = (SHA + "\n").encode()
    files[f"rel/{NAME}"] = fake(SHA)
    files[f"rel/{NAME}.sha256"] = f"{hashlib.sha256(files[f'rel/{NAME}']).hexdigest()}  {NAME}\n".encode()
    yield files, seen
    httpd.shutdown()


def test_newest_build_is_downloaded_verified_and_its_path_printed(tmp_path, server, capsys):
    assert cp.main(["--dir", str(tmp_path)]) == 0
    out = capsys.readouterr()
    assert out.out.strip() == str(tmp_path / NAME) and (tmp_path / NAME).stat().st_mode & 0o111
    assert "downloading" in out.err


def test_explicit_commit_json_and_second_run_downloads_no_binary(tmp_path, server, capsys):
    files, seen = server
    assert cp.main(["--commit", SHA.upper(), "--dir", str(tmp_path), "--json"]) == 0
    d = json.loads(capsys.readouterr().out)
    assert d["schema"] == "cli-prebuilt/1" and d["commit"] == SHA and d["sha256"] == hashlib.sha256(files[f"rel/{NAME}"]).hexdigest()
    seen.clear()
    assert cp.main(["--commit", SHA, "--dir", str(tmp_path)]) == 0
    assert f"/rel/{NAME}" not in seen and "already present" in capsys.readouterr().err


def test_unknown_commit_exits_4(tmp_path, server, capsys):
    assert cp.main(["--commit", "f" * 40, "--dir", str(tmp_path)]) == 4
    assert "older than the last 10 commits" in capsys.readouterr().err


def test_checksum_differs_exits_7_and_writes_nothing(tmp_path, server):
    files, _ = server
    files[f"rel/{NAME}"] += b"# tampered\n"
    assert cp.main(["--commit", SHA, "--dir", str(tmp_path)]) == 7
    assert not list(tmp_path.iterdir())


def test_binary_of_another_commit_exits_7_and_is_deleted(tmp_path, server):
    files, _ = server
    files[f"rel/{NAME}"] = fake("f" * 40)
    files[f"rel/{NAME}.sha256"] = f"{hashlib.sha256(files[f'rel/{NAME}']).hexdigest()}  {NAME}\n".encode()
    assert cp.main(["--commit", SHA, "--dir", str(tmp_path)]) == 7
    assert not (tmp_path / NAME).exists()


def test_server_down_exits_6(tmp_path, server, monkeypatch):
    monkeypatch.setenv("MZLAB_BASE_URL", "http://127.0.0.1:9/rel")
    assert cp.main(["--dir", str(tmp_path)]) == 6


@pytest.mark.parametrize("args", [["--bogus"], ["--commit", "abc123"], ["--commit", "xyz" * 5]])
def test_wrong_usage_exits_2(tmp_path, server, args):
    assert cp.main([*args, "--dir", str(tmp_path)]) == 2


def test_other_platform_exits_3_without_any_request(tmp_path, server, monkeypatch):
    monkeypatch.setattr(cp.platform, "system", lambda: "Darwin")
    assert cp.main(["--dir", str(tmp_path)]) == 3 and not server[1]


def test_dry_run_downloads_nothing(tmp_path, server, capsys):
    assert cp.main(["--commit", SHA, "--dry-run", "--dir", str(tmp_path)]) == 0
    assert json.loads(capsys.readouterr().out)["urls"]["binary"].endswith(NAME) and not server[1] and not list(tmp_path.iterdir())


def test_workflow_is_a_rolling_prerelease_that_cannot_pass_for_a_release():
    yml = (ROOT / ".github" / "workflows" / "cli-main.yml").read_text(encoding="utf-8")
    assert re.search(r"on:\n  push:\n    branches: \[main\]\n  workflow_dispatch:", yml) and "pull_request" not in yml and "tags:" not in yml
    assert "--prerelease --latest=false" in yml and "NOT a release" in yml and "TAG: cli-prebuilt" in yml
    assert not re.search(r"cli-prebuilt/v\d|tag_name: v", yml) and "secrets." not in yml
    assert yml.count("contents: write") == 1 and yml.split("contents: write")[0].count("  publish:") == 1  # write access only in the publish job
    assert cp.BASE.endswith("/releases/download/cli-prebuilt") and f"mzlab-cli-${{GITHUB_SHA:0:12}}-{cp.ARCH}" in yml
