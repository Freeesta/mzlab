"""Shared pytest set-up."""
import os
import tempfile
from pathlib import Path

# the .mzidx files the tests build go to a folder of their own, not to ~/.cache/mzlab
os.environ.setdefault("MZLAB_CACHE", tempfile.mkdtemp(prefix="mzlab-cache-"))

# the node processes started by the tests get I18N (Italian catalog) before the page scripts
os.environ["NODE_OPTIONS"] = (os.environ.get("NODE_OPTIONS", "") + f" --require={Path(__file__).with_name('node_i18n.js')}").strip()


# ------------------------------------------------------------ the readers of the untargeted tables are Rust (crates/mzlab-feat-import-wasm): the node tests that read a table load them
import shutil  # noqa: E402
import subprocess  # noqa: E402

import pytest  # noqa: E402

_ROOT = Path(__file__).resolve().parent.parent
_IMPORT_TESTS = {"test_untarget": None, "test_untarget_match": None, "test_untarget_export": None, "test_untarget_identity": None, "test_untarget_veri": None,
                 "test_fair": {"test_gnps_package_is_read_back_by_the_untargeted_readers", "test_quant_header_and_rows_follow_the_mzmine_format", "test_subset_edges_and_no_ms2"}}


@pytest.fixture(scope="session")
def _import_pkg(tmp_path_factory):
    """The .wasm of the readers for node (wasm-bindgen --target nodejs); None without cargo, wasm-bindgen or the wasm32 target."""
    if not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")):
        return None
    out = tmp_path_factory.mktemp("import-wasm")
    try:
        for cmd in (["cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-import-wasm"],
                    ["wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_feat_import_wasm.wasm"]):
            subprocess.run(cmd, cwd=_ROOT, check=True, capture_output=True)
    except (subprocess.CalledProcessError, OSError):
        return None
    return out


def _with_pkg(pkg):
    if pkg is None:
        pytest.skip("cargo, wasm-bindgen, the wasm32 target or node not available (the readers of the tables are Rust)")
    mp = pytest.MonkeyPatch()
    mp.setenv("MZLAB_IMPORT_PKG", str(pkg))
    return mp


@pytest.fixture(scope="module", autouse=True)
def _import_env_module(request):
    """Whole modules that read tables (also before their own fixtures run)."""
    if request.module.__name__.rsplit(".", 1)[-1] not in _IMPORT_TESTS or _IMPORT_TESTS[request.module.__name__.rsplit(".", 1)[-1]] is not None:
        yield
        return
    mp = _with_pkg(request.getfixturevalue("_import_pkg"))
    yield
    mp.undo()


@pytest.fixture(autouse=True)
def _import_env_test(request):
    """The tests of a module that otherwise does not need the readers."""
    only = _IMPORT_TESTS.get(request.module.__name__.rsplit(".", 1)[-1])
    if not only or request.node.name not in only:
        yield
        return
    mp = _with_pkg(request.getfixturevalue("_import_pkg"))
    yield
    mp.undo()
