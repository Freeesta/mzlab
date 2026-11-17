---
paths:
  - "docs/agenti/**"
  - "CLAUDE.md"
---
# Docs rules
Everything in the repo is English (code, comments, docs, test names, messages, saved formats): no Italian anywhere.
`docs/agenti/` describe the current state only: rewrite the line you change (no «now», «was», PR numbers, dates). A doc line is at most 600 characters, an area file at most 25 KB (`python3 tools/controlla.py`). Generated files are regenerated with their tool.
