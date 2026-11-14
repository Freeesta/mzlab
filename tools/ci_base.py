"""The commit a CI job diffs against, for changed-file detection.
`python3 tools/ci_base.py <fallback sha>` prints it. On `pull_request` the checkout is the merge of the PR into the CURRENT main, so the PR's own files are
the diff of that merge commit with its first parent (main); the event's `base.sha` is where the PR branched and, once main moved, counts other people's files
as the PR's. Any other event (push, merge_group), or a checkout that is not a merge (needs fetch-depth 2), prints the fallback unchanged."""
import os
import subprocess
import sys


def first_parent() -> str:
    """HEAD^1 when HEAD is a merge commit, else an empty string."""
    try:
        if subprocess.run(["git", "rev-parse", "--verify", "-q", "HEAD^2"], capture_output=True).returncode != 0:
            return ""
        return subprocess.run(["git", "rev-parse", "HEAD^1"], check=True, capture_output=True, encoding="utf-8").stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def base(event: str, fallback: str) -> str:
    return (first_parent() if event == "pull_request" else "") or fallback


if __name__ == "__main__":
    print(base(os.environ.get("GITHUB_EVENT_NAME", ""), sys.argv[1] if len(sys.argv) > 1 else ""))
