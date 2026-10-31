"""Copilot against our own chat PRs, for tools/metriche.py (docs/agenti/metriche.md). One row per closed PR from the GitHub API
(injected `api(path)`, so tests use a fake), stored in `copilot-YYYY-MM.jsonl`; `summary` compares Copilot with our chat PRs of similar
size (<= 5 files) and prints an honest verdict (needs MIN_PRS Copilot PRs; PRs that touch sensitive paths are judged separately).
Standard library only. Judgement calls the API cannot answer (was a flagged bug real?) are added by hand with `--aggiungi`."""
from __future__ import annotations

import re
import statistics
from datetime import datetime, timedelta
from pathlib import Path

from ci_metrics import secs, ts
from metriche_ci import REQUIRED

MIN_PRS = 10        # fewer Copilot PRs than this: no verdict
SMALL_FILES = 5     # the comparison group: chat PRs of at most this many files
COPILOT_BOT = "copilot-swe-agent[bot]"
REVIEW_BOT = "copilot-pull-request-reviewer"
SENSITIVE = (".github/", "tools/", "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock",
             "pyproject.toml", "requirements.txt", "rust-toolchain.toml", "deny.toml")
SAME_BAND = 0.2     # medians within 20 % of each other are "same"


def author_kind(pr: dict) -> str:
    login = ((pr.get("user") or {}).get("login") or "").lower()
    ref = (pr.get("head") or {}).get("ref") or ""
    if login.startswith("copilot") or ref.startswith("copilot/"):
        return "copilot"
    return "chat" if ref.startswith("claude/") else "other"


def sensitive_paths(files: list[str]) -> list[str]:
    return [f for f in files if f.startswith(SENSITIVE) or Path(f).name in SENSITIVE]


def secret_workflows(root: Path) -> set[str]:
    """Names of the workflows whose file reads a secret other than the built-in token."""
    out = set()
    for f in sorted((root / ".github" / "workflows").glob("*.yml")):
        txt = f.read_text(encoding="utf-8")
        if re.search(r"secrets\.(?!GITHUB_TOKEN)\w+|secrets\[", txt):
            m = re.search(r"^name:\s*(.+)$", txt, re.M)
            out.add((m.group(1).strip().strip("\"'") if m else f.stem))
    return out


def commit_runs(sha: str, api) -> list[dict]:
    return api(f"/commits/{sha}/check-runs?per_page=100").get("check_runs", [])


def ci_history(commits: list[dict], created: str, api) -> dict:
    """Opened to first fully green required checks, and the number of red required checks over all commits."""
    green_s, red = None, 0
    for c in commits:
        runs = {r["name"]: r for r in commit_runs(c["sha"], api) if r["name"] in REQUIRED}
        red += sum(r.get("conclusion") in ("failure", "timed_out") for r in runs.values())
        if green_s is None and len(runs) == len(REQUIRED) and all(r.get("conclusion") == "success" for r in runs.values()):
            green_s = secs(created, max(r["completed_at"] for r in runs.values()))
    return {"first_green_s": green_s, "red": red}


def review_stats(n: int, commits: list[dict], api) -> dict:
    """Comments of the Copilot reviewer and how many were followed by a later commit that touched the same file around that line."""
    comments = [c for c in api(f"/pulls/{n}/comments?per_page=100") if ((c.get("user") or {}).get("login") or "").lower().startswith(REVIEW_BOT)]
    accepted = 0
    for c in comments:
        line = c.get("original_line") or c.get("line") or 0
        for k in commits:
            if k["commit"]["committer"]["date"] <= c["created_at"]:
                continue
            for f in api(f"/commits/{k['sha']}").get("files", []):
                if f["filename"] == c["path"] and touches(f.get("patch", ""), line):
                    accepted += 1
                    break
            else:
                continue
            break
    return {"comments": len(comments), "accepted": accepted}


def touches(patch: str, line: int, slack: int = 3) -> bool:
    """True if a hunk of the patch (old-file ranges) covers `line` within `slack` lines; a patch without hunks counts as a touch."""
    hunks = re.findall(r"^@@ -(\d+)(?:,(\d+))? ", patch, re.M)
    if not hunks or not line:
        return True
    return any(int(a) - slack <= line <= int(a) + int(b or 1) + slack for a, b in hunks)


def followups(n: int, merged_at: str, api) -> dict:
    """Commits on main in the 7 days after the merge that mention this PR (a revert or a follow-up fix)."""
    t = ts(merged_at)
    rows = api(f"/commits?sha=main&since={merged_at}&until={(t + timedelta(days=7)).strftime('%Y-%m-%dT%H:%M:%SZ')}&per_page=100")
    msgs = [r["commit"]["message"].split("\n")[0] for r in rows if re.search(rf"#{n}\b", r["commit"]["message"]) and not r["commit"]["message"].split("\n")[0].endswith(f"(#{n})")]
    return {"followup_7d": len(msgs), "revert_7d": any(m.lower().startswith("revert") for m in msgs)}


def pr_row(n: int, api, secret_wf: set[str] | None = None, now: datetime | None = None) -> dict | None:
    """The row of one closed PR; None for PRs that are neither Copilot nor our chat PRs."""
    pr = api(f"/pulls/{n}")
    kind = author_kind(pr)
    if kind == "other":
        return None
    files = [f["filename"] for f in api(f"/pulls/{n}/files?per_page=100")]
    commits = api(f"/pulls/{n}/commits?per_page=100")
    bots = [str(((c.get("author") or {}).get("login") or c["commit"]["author"]["name"])).lower().startswith("copilot") for c in commits]
    merged = bool(pr.get("merged_at"))
    row = {"v": 1, "kind": "pr", "pr": n, "who": kind, "created": pr["created_at"], "closed": pr.get("closed_at"), "merged": merged,
           "merged_at": pr.get("merged_at"), "to_merge_s": secs(pr["created_at"], pr.get("merged_at")), "files": len(files), "lines": pr.get("additions", 0) + pr.get("deletions", 0),
           "sensitive": sensitive_paths(files), "commits": len(commits), "fix_rounds": max(0, len(commits) - 1),
           "human_fix_commits": sum(1 for b in bots if not b) if kind == "copilot" else None,
           "unchanged": merged and kind == "copilot" and all(bots), **ci_history(commits, pr["created_at"], api)}
    if kind == "copilot":
        row["review"] = review_stats(n, commits, api)
    if secret_wf and commits:
        runs = api(f"/actions/runs?head_sha={pr['head']['sha']}&per_page=50").get("workflow_runs", [])
        row["secret_runs"] = sorted({r["name"] for r in runs if r.get("name") in secret_wf and r.get("conclusion") not in ("action_required", "skipped", None)})
    if merged:
        row.update(followups(n, pr["merged_at"], api))
    return row


def recent_closed(api, since: datetime, known: dict[int, dict], limit: int = 30) -> list[int]:
    """Numbers of PRs closed after `since` that have no row yet, or whose row was written less than 7 days after the merge (follow-ups may be missing)."""
    def stale(r):
        return bool(r.get("merged_at")) and ts(r["recorded"]) < ts(r["merged_at"]) + timedelta(days=7)
    out = []
    for p in api("/pulls?state=closed&sort=updated&direction=desc&per_page=100"):
        if p.get("closed_at") and ts(p["closed_at"]) >= since and (p["number"] not in known or stale(known[p["number"]])):
            out.append(p["number"])
    return out[:limit]


def med(v: list) -> float | None:
    v = [x for x in v if x is not None]
    return float(statistics.median(v)) if v else None


def group(rows: list[dict], who: str, sensitive: bool | None = None) -> list[dict]:
    out = [r for r in rows if r["who"] == who and r["files"] <= SMALL_FILES]
    return out if sensitive is None else [r for r in out if bool(r["sensitive"]) == sensitive]


def verdict(cop: list[dict], ours: list[dict]) -> str:
    """better / same / worse / too few samples: median open-to-merge time first, then reverts and follow-up fixes of merged PRs."""
    if len(cop) < MIN_PRS:
        return f"too few samples yet ({len(cop)} of {MIN_PRS} Copilot PRs)"
    a, b = med([r["to_merge_s"] for r in cop if r["merged"]]), med([r["to_merge_s"] for r in ours if r["merged"]])
    if a is None or b is None:
        return "too few samples yet (no merged PRs in one group)"
    rate = lambda rs, k: sum(bool(r.get(k)) for r in rs if r["merged"]) / max(1, sum(r["merged"] for r in rs))
    damage = rate(cop, "revert_7d") > rate(ours, "revert_7d") or rate(cop, "followup_7d") > rate(ours, "followup_7d") + 0.1
    if a > b * (1 + SAME_BAND) or damage:
        return "worse"
    return "better" if a < b * (1 - SAME_BAND) else "same"


def summary(rows: list[dict]) -> str:
    """Text for the report: rates, medians against our chat PRs of similar size, and one verdict line per group."""
    latest = {r["pr"]: r for r in rows if r.get("kind") == "pr"}
    rows = list(latest.values())
    cop, ours = group(rows, "copilot"), group(rows, "chat")
    allcop = [r for r in rows if r["who"] == "copilot"]
    if not allcop:
        return "Copilot: no Copilot PRs recorded yet."
    merged = [r for r in allcop if r["merged"]]
    com = sum(r["review"]["comments"] for r in allcop if r.get("review"))
    acc = sum(r["review"]["accepted"] for r in allcop if r.get("review"))
    h = lambda s: "-" if s is None else f"{s / 3600:.1f} h"
    lines = [f"Copilot PRs: {len(allcop)} closed, {len(merged)} merged, {sum(not r['merged'] for r in allcop)} closed unmerged; "
             f"merged unchanged (no human/Claude commit): {sum(r['unchanged'] for r in merged)} of {len(merged)}",
             f"Median open to merge: Copilot {h(med([r['to_merge_s'] for r in cop if r['merged']]))} vs our chat PRs {h(med([r['to_merge_s'] for r in ours if r['merged']]))} "
             f"(<= {SMALL_FILES} files; {len(cop)} vs {len(ours)} PRs)",
             f"Median open to first green: Copilot {h(med([r['first_green_s'] for r in cop]))} vs ours {h(med([r['first_green_s'] for r in ours]))}; "
             f"median fix rounds {med([r['fix_rounds'] for r in cop])} vs {med([r['fix_rounds'] for r in ours])}; "
             f"red required checks {sum(r['red'] for r in cop)} vs {sum(r['red'] for r in ours)}",
             f"Reviewer comments accepted (a later commit touched the same file near that line): {acc} of {com}" + (f" ({100 * acc / com:.0f} %)" if com else ""),
             f"Reverts or follow-ups within 7 days of a merge: {sum(bool(r.get('revert_7d')) for r in allcop)} reverts, {sum(bool(r.get('followup_7d')) for r in allcop)} PRs with follow-ups",
             f"Verdict, non-sensitive paths: {verdict(group(rows, 'copilot', False), group(rows, 'chat', False))}",
             f"Verdict, sensitive paths (.github/, tools/, Cargo.*, package files): {verdict(group(rows, 'copilot', True), group(rows, 'chat', True))}"]
    sens = [r for r in allcop if r["sensitive"]]
    lines.append(f"Copilot PRs touching sensitive paths: {len(sens)}" + "".join(f"\n  #{r['pr']}: {', '.join(r['sensitive'][:4])}" for r in sens))
    secret = [r for r in allcop if r.get("secret_runs")]
    lines.append(f"Copilot PRs whose CI ran a workflow that reads secrets: {len(secret)}" + "".join(f"\n  #{r['pr']}: {', '.join(r['secret_runs'])}" for r in secret))
    return "\n".join(lines)
