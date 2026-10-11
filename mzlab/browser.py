"""Entry point of the browser version: this module runs inside Pyodide (see web/browser-worker.js).

The files the student opens live in the memory of the page (Pyodide's file system); nothing is sent anywhere.
"""
from __future__ import annotations

import json
import os
import shutil
from io import BytesIO
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from .api import dispatch
from .app import App
from .bigfiles import too_big
from .i18n import UserError
from .reader import mzidx
from .reader.mzml import Run

WORK = Path("/work/sessione")
PENDING = Path("/work/pendente")        # uploaded files whose index is still being built: not in the list of the files yet
app: App | None = None


INDEX_TMP = "/idxtmp"           # where Python writes the indexes it builds; the worker moves them to OPFS (web/browser-worker.js) and they are mounted from there
_tried: set = set()


def start() -> None:
    global app
    WORK.mkdir(parents=True, exist_ok=True)
    PENDING.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("MZLAB_CACHE", INDEX_TMP)
    mzidx.FLAT = True                                    # the index of "x.mzML" is "x.mzML.mzidx": the real path of a mounted file changes at every visit
    app = App(WORK)


def configure(small: bool = False) -> None:
    """The ceiling of the blocks kept in memory: 32 MB on a tablet or a phone, 64 MB elsewhere."""
    mzidx.set_cache_limit(small=bool(small))
    mzidx.BATCH_PEAKS, mzidx.BUILD_BYTES = 1 << 18, 32 << 20       # the heap of the engine never shrinks: build the indexes in smaller pieces


def build_index(name: str) -> bool:
    """Build the .mzidx of an uploaded mzML now (in the session when the file is open, otherwise with a Run of its own). True if there is an index afterwards."""
    p = WORK / app.safe_name(name)
    try:
        it = next((i for i in (app.session.items if app.session else []) if i.path.name == p.name), None)
        run = it.run if it else Run(p)
        try:
            return bool(run.scans) and bool(run._ensure_index())
        finally:
            if not it:
                run.close()
    except Exception:  # noqa: BLE001 -- the index is an optimisation: the tables are then built in memory when asked for
        return False


def index_inputs(name: str):
    """What the Rust builder needs for the mzML `name` (see mzidx.builder_inputs), or None when there is nothing to build (already indexed, no scans, unreadable)."""
    p = PENDING / app.safe_name(name)
    if not p.exists():
        p = WORK / app.safe_name(name)
    try:
        it = next((i for i in (app.session.items if app.session else []) if i.path.name == p.name), None)
        run = it.run if it else Run(p)
        try:
            if not run.scans or run._idx or p.name in mzidx.REGISTRY:
                return None
            return mzidx.builder_inputs(run)
        finally:
            if not it:
                run.close()
    except Exception:  # noqa: BLE001 -- the Python builder is the fallback
        return None


def new_indexes() -> list:
    """[[mzML name, path]] of the indexes built in the temporary folder and not yet handed to the worker."""
    out, folder = [], Path(mzidx.cache_dir())
    for f in sorted(folder.glob("*.mzidx")):
        name = f.name[:-len(".mzidx")]
        if name not in mzidx.REGISTRY and (WORK / name).exists() and f.name not in _tried:
            _tried.add(f.name)
            out.append([name, str(f)])
    return out


def adopt_index(name: str, path: str) -> bool:
    """The worker put the index of `name` in OPFS and mounted it at `path`: the open files switch to it and the temporary copy goes."""
    ok, seen = True, set()
    for it in (app.session.items if app.session else []):
        if it.path.name == name and id(it.run) not in seen:
            seen.add(id(it.run))
            ok = it.run.use_index(path) and ok
    if ok:
        mzidx.REGISTRY[name] = path
        (Path(mzidx.cache_dir()) / (name + ".mzidx")).unlink(missing_ok=True)
    return ok


def register_indexes(paths: dict) -> None:
    """At the start: the indexes found in OPFS and mounted by the worker, {mzML name: path}."""
    mzidx.FLAT = True
    mzidx.REGISTRY.update(paths)


def forget_index(name: str) -> None:
    """The file `name` was removed or replaced: its index is not used any more."""
    mzidx.REGISTRY.pop(name, None)
    _tried.discard(name + ".mzidx")
    (Path(mzidx.cache_dir()) / (name + ".mzidx")).unlink(missing_ok=True)


def link_big(name: str, target: str, sniff: bool = True):
    """(status, JSON text) after the worker mounted a big file (WORKERFS, no copy in memory) at `target`: link it into the work folder like an upload.
    With sniff=False the link is made in the pending folder only (the worker builds the index before the file is read: `index_inputs`)."""
    try:
        safe = app.safe_name(name)
        dest = (WORK if sniff else PENDING) / safe
        if dest.is_symlink() or dest.exists():
            dest.unlink()
        dest.symlink_to(target)
        if not sniff:
            return 200, "{}"
        (PENDING / safe).unlink(missing_ok=True)
        return 200, json.dumps({"files": app.files(), "methods": app.methods()})
    except MemoryError:
        return 507, json.dumps(too_big())
    except Exception as e:  # noqa: BLE001
        return 400, json.dumps(e.to_json() if isinstance(e, UserError) else {"error": str(e) if isinstance(e, ValueError) else f"{type(e).__name__}: {e}"})


def handle(method: str, url: str, body=None):
    """(status, JSON text) for one request coming from the page, or (status, bytes, content type) for a binary block. body: bytes (JS Uint8Array) or None."""
    u = urlparse(url)
    path = "/" + u.path.lstrip("./")
    q = {k: v[0] for k, v in parse_qs(u.query).items()}
    data = body.to_bytes() if hasattr(body, "to_bytes") else b""
    code, _ctype, out, _hdr = dispatch(app, method, path, q, BytesIO(data), len(data))
    if path == "/api/remove":                     # nothing to recover in a page: free the memory the removed file used
        shutil.rmtree(app.workdir / "_cestino", ignore_errors=True)
    if _ctype.startswith("application/octet-stream"):          # a binary block (api/scanbin): bytes, not text (they become a Uint8Array in the worker)
        return code, out, _ctype
    return code, out.decode("utf-8")
