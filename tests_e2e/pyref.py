"""The Python reference of the tests (mzlab/api.py, in this process): the same files, the same requests as the page, the answers to compare with the ones of the Rust engine.
`evaluate` has the signature of the page's `pg.evaluate(JS, arg)` for the helpers of the tests: a GET (arg = URL), a POST (arg = [URL, body]) or the clock (0)."""
import json
import sys
import tempfile
from io import BytesIO
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402


class PyRef:
    def __init__(self, files=None, samples=None):
        self.tmp = tempfile.TemporaryDirectory()
        self.app = App(Path(self.tmp.name))
        if files:
            self.open(files, samples)

    def _call(self, method, url, data=b""):
        u = urlparse(url)
        q = {k: v[0] for k, v in parse_qs(u.query).items()}
        code, _ctype, out, _hdr = dispatch(self.app, method, "/" + u.path.lstrip("./"), q, BytesIO(data), len(data))
        return {"status": code, "eng": None, "j": json.loads(out.decode("utf-8")) if out[:1] in (b"{", b"[") else out}

    def upload(self, path):
        p = Path(path)
        return self._call("POST", "api/upload?name=" + p.name, p.read_bytes())

    def open(self, files, samples=None):
        for f in files:
            self.upload(f)
        body = {"samples": samples or [{"file": Path(f).name} for f in files]}
        return self._call("POST", "api/explore", json.dumps(body).encode())

    def get(self, url):
        return self._call("GET", url)

    def post(self, url, body):
        return self._call("POST", url, json.dumps(body).encode())

    def evaluate(self, js, arg=None):
        if "performance.now()" in js:
            return 0
        if "method:" in js:
            return self.post(*arg)
        return self.get(arg)
