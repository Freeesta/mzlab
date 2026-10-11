"""Shared pytest set-up."""
import os
import tempfile
from pathlib import Path

# the .mzidx files the tests build go to a folder of their own, not to ~/.cache/mzlab
os.environ.setdefault("MZLAB_CACHE", tempfile.mkdtemp(prefix="mzlab-cache-"))

# the node processes started by the tests get I18N (Italian catalog) before the page scripts
os.environ["NODE_OPTIONS"] = (os.environ.get("NODE_OPTIONS", "") + f" --require={Path(__file__).with_name('node_i18n.js')}").strip()
