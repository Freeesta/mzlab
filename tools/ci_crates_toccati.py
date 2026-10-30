"""Which workspace crates does a diff reach? Used by rust.yml (`changes` job): prints `affected=<crate> <crate> ...` or `affected=ALL` when a file outside crates/
that every crate depends on (lockfile, manifests, toolchain, vendor, .cargo) changed. A crate is reached when its own folder changed or it depends, directly or not,
on a changed crate (workspace path dependencies from `cargo metadata --no-deps`, dev-dependencies included). Usage: ci_crates_toccati.py BASE HEAD."""
import json
import re
import subprocess
import sys

GLOBALE = re.compile(r"^(Cargo\.(toml|lock)|rust-toolchain\.toml|vendor/|\.cargo/|crates/Cargo\.)")


def main(base: str, head: str) -> str:
    files = subprocess.run(["git", "diff", "--name-only", base, head], capture_output=True, text=True, check=True).stdout.split()
    if any(GLOBALE.match(f) for f in files):
        return "ALL"
    meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], capture_output=True, text=True, check=True).stdout)
    root = meta["workspace_root"].rstrip("/") + "/"
    cartelle = {p["name"]: p["manifest_path"][len(root):].rsplit("/", 1)[0] + "/" for p in meta["packages"]}
    # a crate manifest outside its folder prefix (root package) is not expected: treat an unknown change under crates/ as global
    if any(f.startswith("crates/") and not any(f.startswith(c) for c in cartelle.values()) for f in files):
        return "ALL"
    toccati = {n for n, c in cartelle.items() if any(f.startswith(c) for f in files)}
    dip = {p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in cartelle} for p in meta["packages"]}
    cambiato = True
    while cambiato:
        cambiato = False
        for n, ds in dip.items():
            if n not in toccati and ds & toccati:
                toccati.add(n)
                cambiato = True
    return " ".join(sorted(toccati))


if __name__ == "__main__":
    print(main(sys.argv[1], sys.argv[2]))
