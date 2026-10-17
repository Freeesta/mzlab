# Security policy

mzLab runs entirely in the browser (or in the desktop app): the files of the students never leave their computer. This page says how to report a vulnerability and what to expect.

## Reporting a vulnerability

Please **do not open a public issue** for a security problem. Use the private form of GitHub instead:

1. Open <https://github.com/Freeesta/mzlab/security/advisories/new> (tab *Security* → *Report a vulnerability*).
2. Describe what you found, how to reproduce it and which version or page address is affected.
3. Never attach real laboratory or personal data: a synthetic file or a description is enough.

This is a teaching project maintained on a best-effort basis: we read every report and aim to answer within a few weeks, but we cannot promise a deadline. When a fix is released, the advisory is published and the reporter is credited unless they prefer not to be.

## Supported versions

Only the latest release of the desktop app and the version of the site published at <https://freeesta.github.io/mzlab/> receive security fixes.

## What the project already does

- A Content-Security-Policy in the page (no `unsafe-eval`, network only to the same origin) and in the desktop app.
- The desktop updates are checked by SHA-256.
- Dependencies are watched by Dependabot (Cargo, pip, GitHub Actions) and checked by `cargo-deny` (advisories and licences).
- A CycloneDX software bill of materials (SBOM) is attached to every release.
