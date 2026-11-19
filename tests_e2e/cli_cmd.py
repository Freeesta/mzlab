"""The command that starts mzlab-cli in an e2e test: the release binary tools/verifica.py built once (MZLAB_CLI), else `cargo run` (a stand-alone run of one test).
A `cargo run` per call inside the timeout of a test hung in the CI (a rebuild, or the lock of the build folder): the built binary has neither."""
import os


def cli_command() -> list[str]:
    exe = os.environ.get("MZLAB_CLI")
    return [exe] if exe else ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]
