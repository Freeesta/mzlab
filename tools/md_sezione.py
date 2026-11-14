"""Replace one `## ` section of a markdown file and keep the others: used by tools that write their own measurements into a
docs/agenti area file (tools/bench.py --librerie --scrivi). No dependency, so the tests import it without Playwright."""


def replace_section(txt: str, head: str, sec: list[str]) -> str:
    """`txt` with the section that starts with `head` replaced by `sec`, up to the next `## ` heading: the sections after it stay."""
    i = txt.find("\n" + head)
    if i < 0:
        return txt.rstrip("\n") + "\n\n" + "\n".join(sec) + "\n"
    j = txt.find("\n## ", i + 1)
    rest = "" if j < 0 else "\n" + txt[j + 1:]
    return txt[:i].rstrip("\n") + "\n\n" + "\n".join(sec) + "\n" + rest
