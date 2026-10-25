// @ts-strict
// Pull-request client of the Teoria editor (public repository Freeesta/mzlab): reads a chapter from main and sends an edit as ONE pull request
// (new branch, the .md and its generated .html, auto-merge once the checks are green). One editor pull request may be open at a time; main is never written directly.
// fetch is injectable so the whole flow is tested in Node (tests/test_teoria_editor.py).
import { GhError, b64dec, b64enc, makeCall } from "./gh.js";

export const REPO = { owner: "Freeesta", repo: "mzlab", base: "main" };
const SRC = "mzlab/web/teoria/src";
const PAGES = "mzlab/web/teoria";
const INDEX = `${PAGES}/indice-ricerca.js`;
export const BRANCH_PREFIX = "libro/teoria-";
const API = "https://api.github.com";

/** @param {string} p */
const enc = (p) => p.split("/").map(encodeURIComponent).join("/");

/** Chapter number and file stem of a source name: `12-dati.md` -> `12-dati`. @param {string} name */
export const stem = (name) => name.replace(/\.md$/, "");

/** Heading lines (`#`..`###`) of a source, in order: an edit that changes them needs a regenerated search index, which the editor cannot do. @param {string} md */
export function headings(md) {
  return md.split("\n").filter((l) => /^#{1,3} /.test(l)).map((l) => l.trimEnd());
}

/** Switch on auto-merge (squash) for a pull request: false when GitHub refuses (it may not be allowed in the repository settings). @param {ReturnType<typeof makeCall>} call @param {string} nodeId */
async function autoMerge(call, nodeId) {
  try {
    const g = await call(`${API}/graphql`, {
      method: "POST",
      body: JSON.stringify({ query: "mutation($id: ID!) { enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: SQUASH}) { pullRequest { number } } }", variables: { id: nodeId } }),
    });
    return !(g.errors && g.errors.length);
  } catch (e) {
    return false;
  }
}

/** @param {{token: string, fetchFn?: typeof fetch, repo?: typeof REPO, now?: () => Date}} o */
export function makePrClient({ token, fetchFn = (...a) => fetch(...a), repo = REPO, now = () => new Date() }) {
  const call = makeCall(token, fetchFn);
  const base = `${API}/repos/${repo.owner}/${repo.repo}`;

  /** File text and blob sha on a ref, or null when missing. @param {string} path @param {string} ref */
  async function read(path, ref) {
    try {
      const f = await call(`${base}/contents/${enc(path)}?ref=${encodeURIComponent(ref)}`);
      return { text: b64dec(f.content), sha: /** @type {string} */ (f.sha) };
    } catch (e) {
      if (e instanceof GhError && e.kind === "notfound") return null;
      throw e;
    }
  }

  /** Open pull requests of the editor (head branch starting with BRANCH_PREFIX). */
  async function openPrs() {
    const l = await call(`${base}/pulls?state=open&base=${repo.base}&per_page=100`);
    return /** @type {any[]} */ (l)
      .filter((p) => String(p.head?.ref || "").startsWith(BRANCH_PREFIX))
      .map((p) => ({ number: /** @type {number} */ (p.number), url: /** @type {string} */ (p.html_url), title: /** @type {string} */ (p.title) }));
  }

  return {
    /** Source names of the chapters (`12-dati.md`), sorted. */
    async chapters() {
      const t = await call(`${base}/git/trees/${encodeURIComponent(repo.base)}?recursive=1`);
      const pre = `${SRC}/`;
      return /** @type {{path: string}[]} */ (t.tree)
        .map((n) => n.path)
        .filter((p) => p.startsWith(pre) && /^\d\d-[a-z0-9-]+\.md$/.test(p.slice(pre.length)))
        .map((p) => p.slice(pre.length))
        .sort();
    },
    /** The source, its generated page and the list of imported chapters, all from main. @param {string} name */
    async load(name) {
      const [md, html, listed, index] = await Promise.all([
        read(`${SRC}/${name}`, repo.base),
        read(`${PAGES}/${stem(name)}.html`, repo.base),
        read("tools/libro/importati.txt", repo.base),
        read(INDEX, repo.base),
      ]);
      if (!md || !html || !index) throw new GhError("notfound", `${name} is not on ${repo.base}.`, 404);
      const imported = (listed ? listed.text : "").split("\n").map((l) => l.split("#")[0].trim()).filter(Boolean);
      return { md, html, index, rawOk: imported.includes(name) };
    },
    openPrs,
    /**
     * Send the edit: branch from main, commit the source and its page, open the pull request, switch on auto-merge.
     * `loaded` = the blobs read by load(); if main moved on those files since, nothing is written (GhError "conflict").
     * `index` = the rebuilt search index when a section was retitled, with the blob it was made from.
     * @param {{name: string, md: string, html: string, loaded: {md: {sha: string}, html: {sha: string}}, summary: string, index?: {text: string, loadedSha: string}}} e
     */
    async submit({ name, md, html, loaded, summary, index }) {
      const open = await openPrs();
      if (open.length) throw new GhError("conflict", `Pull request #${open[0].number} of the editor is still open. Wait until it is merged.`, 409);
      const mdPath = `${SRC}/${name}`;
      const htmlPath = `${PAGES}/${stem(name)}.html`;
      const [curMd, curHtml, curIndex] = await Promise.all([read(mdPath, repo.base), read(htmlPath, repo.base), index ? read(INDEX, repo.base) : Promise.resolve(null)]);
      const indexMoved = !!index && (!curIndex || curIndex.sha !== index.loadedSha);
      if (!curMd || !curHtml || indexMoved || curMd.sha !== loaded.md.sha || curHtml.sha !== loaded.html.sha) throw new GhError("conflict", "The chapter changed on main while you were editing. Nothing was written.", 409);
      const head = await call(`${base}/git/ref/heads/${repo.base}`);
      const stamp = now().toISOString().replace(/[-:T]/g, "").slice(0, 12);
      const branch = `${BRANCH_PREFIX}${stem(name)}-${stamp}`;
      await call(`${base}/git/refs`, { method: "POST", body: JSON.stringify({ ref: `refs/heads/${branch}`, sha: head.object.sha }) });
      const msg = `Teoria ${stem(name)}: ${summary}`;
      await call(`${base}/contents/${enc(mdPath)}`, { method: "PUT", body: JSON.stringify({ message: msg, content: b64enc(md), branch, sha: curMd.sha }) });
      await call(`${base}/contents/${enc(htmlPath)}`, { method: "PUT", body: JSON.stringify({ message: `${msg} (generated page)`, content: b64enc(html), branch, sha: curHtml.sha }) });
      if (index && curIndex) {
        await call(`${base}/contents/${enc(INDEX)}`, { method: "PUT", body: JSON.stringify({ message: `${msg} (search index)`, content: b64enc(index.text), branch, sha: curIndex.sha }) });
      }
      const pr = await call(`${base}/pulls`, {
        method: "POST",
        body: JSON.stringify({ title: msg, head: branch, base: repo.base, body: `Edit of the Teoria chapter ${stem(name)} made with the Teoria editor (source and generated page; CI checks the page against the Python converter).\n\n${summary}` }),
      });
      const auto = await autoMerge(call, pr.node_id);
      return { number: /** @type {number} */ (pr.number), url: /** @type {string} */ (pr.html_url), branch, autoMerge: auto };
    },
  };
}
