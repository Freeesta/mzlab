// @ts-strict
// GitHub Contents API client for the private Libro repository. Admin tool of the editor page: the token and the text live in memory only.
// Every call goes to api.github.com (the only host the page CSP allows); fetch is injectable so the logic is tested in Node.

export const REPO = { owner: "Freeesta", repo: "mzlab-dati", branch: "main", dir: "Libro_Untargeted" };
const API = "https://api.github.com";

export class GhError extends Error {
  /** @param {"auth"|"conflict"|"notfound"|"network"|"other"} kind @param {string} message @param {number} [status] */
  constructor(kind, message, status = 0) {
    super(message);
    this.kind = kind;
    this.status = status;
  }
}

/** @param {string} s */
export function b64enc(s) {
  const bytes = new TextEncoder().encode(s);
  let bin = "";
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(bin);
}

/** @param {string} b */
export function b64dec(b) {
  const bin = atob(b.replace(/\s/g, ""));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

/** Names of the three files of a section: the body, the personal notes and the questions and answers. @param {string} bodyPath folder/X.md */
export function trio(bodyPath) {
  const stem = bodyPath.replace(/\.md$/, "");
  return { text: bodyPath, notes: `${stem}.appunti.md`, qa: `${stem}.qa.md` };
}

/** True for the body files of the sections (not the notes, the Q&A or the dialect guide). @param {string} p */
export function isSection(p) {
  return /\.md$/.test(p) && !/\.(appunti|qa)\.md$/.test(p) && !/(^|\/)(LEGGIMI|README)\.md$/i.test(p);
}

/** The authenticated fetch of the GitHub API, shared by the Libro client here and the Teoria pull-request client (pr.js). Errors are GhError. @param {string} token @param {typeof fetch} fetchFn */
export function makeCall(token, fetchFn) {
  /** @param {string} url @param {RequestInit} [init] */
  async function call(url, init = {}) {
    let res;
    try {
      res = await fetchFn(url, {
        ...init,
        cache: "no-store",
        credentials: "omit",
        headers: { Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28", Authorization: `Bearer ${token}`, ...(init.headers || {}) },
      });
    } catch (e) {
      throw new GhError("network", "Cannot reach GitHub. Check the connection.");
    }
    if (res.ok) return res.json();
    const body = await res.json().catch(() => ({}));
    const msg = String(body.message || res.statusText || res.status);
    if (res.status === 401 || res.status === 403) throw new GhError("auth", `GitHub refused the token (${msg}). It may have expired.`, res.status);
    if (res.status === 404) throw new GhError("notfound", "Not found.", 404);
    if (res.status === 409 || res.status === 422) throw new GhError("conflict", msg, res.status);
    throw new GhError("other", msg, res.status);
  }
  return call;
}

/**
 * @param {{token: string, fetchFn?: typeof fetch, repo?: typeof REPO}} o
 */
export function makeClient({ token, fetchFn = (...a) => fetch(...a), repo = REPO }) {
  const base = `${API}/repos/${repo.owner}/${repo.repo}`;

  const call = makeCall(token, fetchFn);

  /** @param {string} p */
  const enc = (p) => p.split("/").map(encodeURIComponent).join("/");
  // The data repository moved the book from Libro_Untargeted/ to libro/testo/: sections() looks at the tree and keeps whichever exists.
  let dir = repo.dir;
  const fp = (/** @type {string} */ p) => enc(`${dir}/${p}`);

  return {
    /** Paths of the section files under the Libro folder (relative to it), sorted. */
    async sections() {
      const t = await call(`${base}/git/trees/${encodeURIComponent(repo.branch)}?recursive=1`);
      const paths = /** @type {{path: string}[]} */ (t.tree).map((n) => n.path);
      if (paths.some((p) => p.startsWith("libro/testo/"))) dir = "libro/testo";
      const pre = `${dir}/`;
      return paths
        .filter((p) => p.startsWith(pre))
        .map((p) => p.slice(pre.length))
        .filter(isSection)
        .sort();
    },
    /** File text and blob sha, or null when the file does not exist yet. @param {string} p @param {string} [ref] */
    async read(p, ref = repo.branch) {
      try {
        const f = await call(`${base}/contents/${fp(p)}?ref=${encodeURIComponent(ref)}`);
        return { text: b64dec(f.content), sha: /** @type {string} */ (f.sha) };
      } catch (e) {
        if (e instanceof GhError && e.kind === "notfound") return null;
        throw e;
      }
    },
    /** Commit one file to the branch. sha = the blob read before ("" for a new file); a stale sha gives GhError "conflict". @param {string} p @param {string} text @param {string} sha @param {string} message */
    async write(p, text, sha, message) {
      const body = { message, content: b64enc(text), branch: repo.branch, ...(sha ? { sha } : {}) };
      const r = await call(`${base}/contents/${fp(p)}`, { method: "PUT", body: JSON.stringify(body) });
      return { sha: /** @type {string} */ (r.content.sha), commit: /** @type {string} */ (r.commit.sha) };
    },
    /** Latest commits that touched the file. @param {string} p @param {number} [n] */
    async history(p, n = 20) {
      const l = await call(`${base}/commits?sha=${encodeURIComponent(repo.branch)}&path=${fp(p)}&per_page=${n}`);
      return /** @type {any[]} */ (l).map((c) => ({ sha: /** @type {string} */ (c.sha), date: /** @type {string} */ (c.commit.committer.date), message: /** @type {string} */ (c.commit.message.split("\n")[0]), url: /** @type {string} */ (c.html_url) }));
    },
  };
}
