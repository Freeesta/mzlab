"""The reference answers of the e2e tests, from committed golden data (tests/golden/e2e/<test>.json.gz): same interface as `pyref.PyRef`, no Python engine in the process.
Every call is keyed by the chain of the calls made before it on the same object (method, URL, body, SHA-256 of the uploaded bytes), so the files may live anywhere.
To rewrite the data (a deliberate change of the reference): `MZLAB_REGEN_GOLDEN=1 python3 tools/verifica.py --solo <test>`; that is the only case that loads pyref.py."""
import atexit
import base64
import gzip
import hashlib
import json
import os
import sys
from pathlib import Path

DIR = Path(__file__).resolve().parent.parent / "tests" / "golden" / "e2e"
REGEN = bool(os.environ.get("MZLAB_REGEN_GOLDEN"))
_store = {}


def _name():
    return Path(sys.argv[0]).stem or "test"


def _load():
    n = _name()
    if n not in _store:
        f = DIR / (n + ".json.gz")
        _store[n] = {} if REGEN or not f.is_file() else json.loads(gzip.decompress(f.read_bytes()))
        if REGEN:
            atexit.register(_save, n)
    return _store[n]


def _save(n):
    DIR.mkdir(parents=True, exist_ok=True)
    (DIR / (n + ".json.gz")).write_bytes(gzip.compress(json.dumps(_store[n], sort_keys=True, separators=(",", ":")).encode(), 9, mtime=0))


def _enc(r):
    j = r["j"]
    return {"status": r["status"], "eng": r["eng"], "j": {"b64": base64.b64encode(j).decode()} if isinstance(j, bytes) else {"json": j}}


def _dec(e):
    j = e["j"]
    return {"status": e["status"], "eng": e["eng"], "j": base64.b64decode(j["b64"]) if "b64" in j else j["json"]}


class PyRef:
    def __init__(self, files=None, samples=None):
        self.chain = hashlib.sha256(b"golden/1").hexdigest()
        self.real = None
        if REGEN:
            sys.path.insert(0, str(Path(__file__).resolve().parent))
            import pyref
            self.real = pyref.PyRef()
        if files:
            self.open(files, samples)

    def _step(self, tag, call, *parts):
        self.chain = hashlib.sha256(json.dumps([self.chain, tag, *parts], sort_keys=True).encode()).hexdigest()
        st = _load()
        if REGEN:
            st[self.chain] = _enc(call())
        if self.chain not in st:
            raise KeyError(f"no golden answer for {tag} {parts[0]!r} in {_name()}: regenerate with MZLAB_REGEN_GOLDEN=1")
        return _dec(st[self.chain])

    def upload(self, path):
        p = Path(path)
        return self._step("upload", lambda: self.real.upload(path), p.name, hashlib.sha256(p.read_bytes()).hexdigest())

    def open(self, files, samples=None):
        for f in files:
            self.upload(f)
        body = {"samples": samples or [{"file": Path(f).name} for f in files]}
        return self._step("open", lambda: self.real.open([], samples or [{"file": Path(f).name} for f in files]), body)

    def get(self, url):
        return self._step("get", lambda: self.real.get(url), url)

    def post(self, url, body):
        return self._step("post", lambda: self.real.post(url, body), url, body)

    def evaluate(self, js, arg=None):
        if "performance.now()" in js:
            return 0
        if "method:" in js:
            return self.post(*arg)
        return self.get(arg)
