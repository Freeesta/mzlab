// Fake GitHub for the pull-request flow of the Teoria editor (mzlab/web/libro/pr.js); prints one JSON object with what each scenario saw. See tests/test_teoria_editor.py.
import { makePrClient, headings } from "../mzlab/web/libro/pr.js";
import { b64enc, b64dec } from "../mzlab/web/libro/gh.js";

const SRC = "mzlab/web/teoria/src/12-dati.md";
const PAGE = "mzlab/web/teoria/12-dati.html";

function fake({ open = [], moved = false, graphql = "ok" } = {}) {
  const calls = [];
  const files = { [SRC]: { text: "# Dati\n\nUno.\n", sha: "m1" }, [PAGE]: { text: "<p>Uno</p>", sha: "h1" }, "tools/libro/importati.txt": { text: "12-dati.md\n# x\n", sha: "i1" } };
  if (moved) files[SRC].sha = "m2";
  const json = (o, status = 200) => ({ ok: status < 300, status, statusText: "", json: async () => o });
  const fn = async (url, init = {}) => {
    const u = new URL(url);
    const method = init.method || "GET";
    const body = init.body ? JSON.parse(init.body) : null;
    calls.push({ method, path: u.pathname + u.search, body });
    if (u.pathname.endsWith("/git/trees/main")) return json({ tree: [{ path: "mzlab/web/teoria/src/12-dati.md" }, { path: "mzlab/web/teoria/src/00-uso.md" }, { path: "mzlab/web/teoria/src/LEGGIMI.md" }, { path: "mzlab/web/teoria/src/sub/x.md" }, { path: "mzlab/web/teoria/12-dati.html" }] });
    const c = u.pathname.match(/\/contents\/(.+)$/);
    if (c && method === "GET") {
      const f = files[decodeURIComponent(c[1])];
      return f ? json({ content: b64enc(f.text), sha: f.sha }) : json({ message: "Not Found" }, 404);
    }
    if (c && method === "PUT") return json({ content: { sha: "new" }, commit: { sha: "c" } });
    if (u.pathname.endsWith("/pulls") && method === "GET") return json(open);
    if (u.pathname.endsWith("/git/ref/heads/main")) return json({ object: { sha: "tip" } });
    if (u.pathname.endsWith("/git/refs")) return json({}, 201);
    if (u.pathname.endsWith("/pulls") && method === "POST") return json({ number: 7, html_url: "https://github.com/Freeesta/mzlab/pull/7", node_id: "PR_7" }, 201);
    if (u.pathname === "/graphql") return graphql === "ok" ? json({ data: {} }) : json({ errors: [{ message: "Auto merge is not allowed" }] });
    return json({ message: "unexpected " + url }, 500);
  };
  return { fn, calls };
}

const out = {};
const edit = { name: "12-dati.md", md: "# Dati\n\nDue.\n", html: "<p>Due</p>", loaded: { md: { sha: "m1" }, html: { sha: "h1" } }, summary: "fix a typo" };
const now = () => new Date("2026-10-20T14:05:30Z");

{
  const g = fake();
  const c = makePrClient({ token: "t", fetchFn: g.fn, now });
  out.chapters = await c.chapters();
  out.load = await c.load("12-dati.md");
  const r = await c.submit(edit);
  out.ok = { r, calls: g.calls.filter((x) => x.method !== "GET").map((x) => ({ m: x.method, p: x.path.replace("/repos/Freeesta/mzlab", ""), b: x.body && { ...x.body, content: x.body.content && b64dec(x.body.content) } })) };
}
{
  const g = fake({ open: [{ number: 5, html_url: "u", title: "t", head: { ref: "libro/teoria-12-dati-202610200000" } }, { number: 6, html_url: "u", title: "t", head: { ref: "other" } }] });
  const c = makePrClient({ token: "t", fetchFn: g.fn, now });
  out.openBlocks = await c.submit(edit).then(() => "sent", (e) => e.kind + ": " + e.message);
  out.openWrites = g.calls.filter((x) => x.method !== "GET").length;
}
{
  const g = fake({ moved: true });
  const c = makePrClient({ token: "t", fetchFn: g.fn, now });
  out.stale = await c.submit(edit).then(() => "sent", (e) => e.kind);
  out.staleWrites = g.calls.filter((x) => x.method !== "GET").length;
}
{
  const g = fake({ graphql: "err" });
  const c = makePrClient({ token: "t", fetchFn: g.fn, now });
  out.noAuto = (await c.submit(edit)).autoMerge;
}
out.headings = headings("# A\n\ntext\n## B {#b}\n#### no\n### C\n#nospace\n");
console.log(JSON.stringify(out));
