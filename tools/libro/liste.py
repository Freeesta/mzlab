"""Lists of the Libro dialect (- and 1.), with indented blocks inside an item: a mixin of the block Parser in blocchi.py."""
import re

ITEM = re.compile(r"^([-*]|\d+\.) +(.*)$")


class Liste:
    def listing(self, lines, i, base):
        tag = "ol" if lines[i][0].isdigit() else "ul"
        items, j = [], i
        while j < len(lines):
            m = ITEM.match(lines[j])
            if not m or m.group(1)[0].isdigit() != (tag == "ol"):
                break
            body, k = [m.group(2)], j + 1
            while k < len(lines) and (lines[k].startswith("  ") or (not lines[k].strip() and self.indented_next(lines, k))):
                body.append(lines[k][3:] if lines[k].startswith("   ") else lines[k].strip())
                k += 1
            items.append((body, j))
            j = k
            while j < len(lines) and not lines[j].strip() and j + 1 < len(lines) and ITEM.match(lines[j + 1]):
                j += 1
        return f"<{tag}>\n" + "\n".join(self.item(b, base + s) for b, s in items) + f"\n</{tag}>", j

    @staticmethod
    def indented_next(lines, k):
        nxt = next((x for x in lines[k + 1:] if x.strip()), "")
        return nxt.startswith("  ")

    def item(self, body, at):
        inner = self.blocks(body, at)
        simple = inner.startswith("<p>") and inner.count("<p>") == 1
        return f"<li>{inner[3:-4] if simple else inner}</li>"
