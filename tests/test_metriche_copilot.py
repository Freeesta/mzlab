"""tools/metriche_copilot.py with a fake GitHub API: Copilot against chat PRs, sensitive paths, reviews, verdicts, storage."""
import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import metriche as m  # noqa: E402
import metriche_copilot as c  # noqa: E402
from metriche_ci import REQUIRED  # noqa: E402

T0 = datetime(2026, 10, 30, 8, tzinfo=timezone.utc)
iso = lambda dt: dt.strftime("%Y-%m-%dT%H:%M:%SZ")


def commit(sha, who, dt):
    return {"sha": sha, "author": {"login": who}, "commit": {"author": {"name": who}, "committer": {"date": iso(dt)}, "message": "m"}}


class Fake:
    """PR n: author by `kind`, files, commits [(who, minutes)], required checks green from the first commit unless `red`."""
    def __init__(self, prs):
        self.prs = prs

    def __call__(self, path):
        p = path.split("?")[0]
        if p == "/pulls":
            return [{"number": n, "closed_at": iso(T0 + timedelta(hours=pr["merge_h"] if pr["merge_h"] else 1))} for n, pr in self.prs.items()]
        parts = p.strip("/").split("/")
        if parts[0] == "pulls":
            n = int(parts[1])
            pr = self.prs[n]
            if len(parts) == 2:
                login = "copilot-swe-agent[bot]" if pr["kind"] == "copilot" else "Freeesta"
                ref = ("copilot/x" if pr["kind"] == "copilot" else "claude/x") if pr["kind"] != "human" else "feature"
                merged = pr["merge_h"] is not None
                return {"number": n, "user": {"login": login}, "head": {"ref": ref, "sha": f"h{n}"}, "created_at": iso(T0), "closed_at": iso(T0 + timedelta(hours=pr["merge_h"] or 1)),
                        "merged_at": iso(T0 + timedelta(hours=pr["merge_h"])) if merged else None, "additions": 30, "deletions": 5}
            if parts[2] == "files":
                return [{"filename": f} for f in pr["files"]]
            if parts[2] == "commits":
                who = "copilot-swe-agent[bot]" if pr["kind"] == "copilot" else "Freeesta"
                return [commit(f"c{n}_{i}", w or who, T0 + timedelta(minutes=10 * (i + 1))) for i, w in enumerate(pr["commits"])]
            if parts[2] == "comments":
                return pr.get("comments", [])
        if parts[0] == "commits" and len(parts) == 3 and parts[2] == "check-runs":
            n = int(parts[1][1:].split("_")[0])
            red = self.prs[n].get("red", 0)
            return {"check_runs": [{"name": r, "conclusion": "failure" if i < red else "success", "completed_at": iso(T0 + timedelta(minutes=30))} for i, r in enumerate(REQUIRED)]}
        if parts[0] == "commits" and len(parts) == 2:
            return {"files": [{"filename": "a.py", "patch": "@@ -10,3 +10,4 @@"}]}
        if p == "/commits":
            return self.prs.get("followups", [])
        if parts[0] == "actions":
            return {"workflow_runs": [{"name": "test", "conclusion": "success"}]}
        raise AssertionError(path)


def prs(n_cop, cop_h, n_chat, chat_h):
    d = {}
    for i in range(n_cop):
        d[100 + i] = {"kind": "copilot", "files": ["a.py"], "commits": [None], "merge_h": cop_h}
    for i in range(n_chat):
        d[200 + i] = {"kind": "chat", "files": ["b.py"], "commits": [None, None], "merge_h": chat_h}
    return d


def test_row_fields_and_sensitive():
    api = Fake({1: {"kind": "copilot", "files": ["x.py", ".github/workflows/t.yml", "Cargo.lock"], "commits": [None, "Freeesta"], "merge_h": 2, "red": 1}})
    r = c.pr_row(1, api, {"test"})
    assert r["who"] == "copilot" and r["merged"] and r["to_merge_s"] == 7200 and r["files"] == 3 and r["lines"] == 35
    assert r["sensitive"] == [".github/workflows/t.yml", "Cargo.lock"] and r["fix_rounds"] == 1 and r["human_fix_commits"] == 1 and r["unchanged"] is False
    assert r["red"] == 2 and r["first_green_s"] is None      # one red required check on each of the two commits, never all green
    assert r["secret_runs"] == ["test"] and r["revert_7d"] is False
    assert c.pr_row(2, Fake({2: {"kind": "human", "files": ["a"], "commits": [None], "merge_h": 1}}), None) is None
    clean = c.pr_row(1, Fake({1: {"kind": "copilot", "files": ["a.py"], "commits": [None], "merge_h": 1}}), None)
    assert clean["unchanged"] is True and clean["first_green_s"] == 1800 and clean["red"] == 0 and "secret_runs" not in clean


def test_review_accepted_when_a_later_commit_touches_the_line():
    comments = [{"user": {"login": "copilot-pull-request-reviewer[bot]"}, "created_at": iso(T0 + timedelta(minutes=15)), "path": "a.py", "line": 11},
                {"user": {"login": "copilot-pull-request-reviewer[bot]"}, "created_at": iso(T0 + timedelta(minutes=15)), "path": "other.py", "line": 1},
                {"user": {"login": "someone"}, "created_at": iso(T0), "path": "a.py", "line": 11}]
    api = Fake({1: {"kind": "copilot", "files": ["a.py"], "commits": [None, None, None], "merge_h": 1, "comments": comments}})
    assert c.pr_row(1, api, None)["review"] == {"comments": 2, "accepted": 1}
    assert c.touches("@@ -10,3 +10,4 @@", 11) and not c.touches("@@ -10,3 +10,4 @@", 80)


def test_followups_and_revert():
    n = 1
    api = Fake({n: {"kind": "copilot", "files": ["a"], "commits": [None], "merge_h": 1}, "followups": [
        {"commit": {"message": "Fix the thing from #1 (#9)"}}, {"commit": {"message": "Revert \"Add x (#1)\"\n\nThis reverts #1"}}, {"commit": {"message": "Add x (#1)"}}]})
    r = c.pr_row(n, api, None)
    assert r["followup_7d"] == 2 and r["revert_7d"] is True


def test_verdicts():
    def rows(n_cop, cop_h, n_chat, chat_h):
        return [c.pr_row(k, Fake(prs(n_cop, cop_h, n_chat, chat_h)), None) for k in list(prs(n_cop, cop_h, n_chat, chat_h))]
    assert c.verdict(*[[r for r in rows(3, 1, 12, 2) if r["who"] == w] for w in ("copilot", "chat")]).startswith("too few")
    for cop_h, want in ((1, "better"), (2, "same"), (4, "worse")):
        rs = rows(12, cop_h, 12, 2)
        assert c.verdict(c.group(rs, "copilot"), c.group(rs, "chat")) == want
    big = c.pr_row(1, Fake({1: {"kind": "copilot", "files": list("abcdefg"), "commits": [None], "merge_h": 1}}), None)
    assert c.group([big], "copilot") == []      # more than 5 files: not compared


def test_summary_separates_sensitive_and_storage(tmp_path):
    api_prs = prs(10, 1, 10, 2)
    for k in range(5):
        api_prs[100 + k]["files"] = [".github/workflows/x.yml"]
    api = Fake(api_prs)
    d = str(tmp_path)
    now = T0 + timedelta(hours=5)
    (tmp_path / "wf").mkdir()
    assert m.registra_copilot(d, api, 14, tmp_path, now) == 20
    # rows written less than 7 days after the merge are written again (follow-ups may have come since)
    assert m.registra_copilot(d, api, 14, tmp_path, now) == 20
    txt = c.summary(m.copilot_righe(d))
    assert "Copilot PRs: 10 closed, 10 merged" in txt and "merged unchanged (no human/Claude commit): 10 of 10" in txt
    assert "Verdict, non-sensitive paths: too few samples yet (5 of 10" in txt and "Verdict, sensitive paths" in txt and "touching sensitive paths: 5" in txt
    assert c.summary([]) == "Copilot: no Copilot PRs recorded yet."
    assert len(list(tmp_path.glob("copilot-*.jsonl"))) == 1 and not list(tmp_path.glob("metriche-*"))
