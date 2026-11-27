Read docs/coda/BRIEF.md.

# MENU-LABELS-LEFTOVERS: Short wording for what the menu-labels PR left

**Goal.** The house style for menu and button labels (docs/agenti/languages.md, «Menu and button labels»; check in `tools/controlla_menu.py`) is applied to context menus, palette commands and a few toolbar tooltips. Apply it to the rest, without changing any meaning.

**Read first.** `docs/agenti/languages.md`, `tools/controlla_menu.py` (`MENU_ARTICLE_OK` lists what the first pass kept and why), `docs/inventories/TOOLTIP.md`.

**Scope.** (1) The `*.aria` keys of the menu entries (`ctx.*.aria`): accessible names with articles; keep them as clear as the visible label. (2) Button labels of dialogs and side panels (`Close the settings`, `Start the tour`, `Replay the tour`, `Take the tour`, `Add the ...`) found with `rg -n 'I18N.t\("' mzlab/web` on `title:`/`label:` properties. (3) Help texts that quote an entry by its old name (`help.*.body`): quoted names must equal the new labels. (4) Long dialog texts: list them in the PR body as a table (key, length) and shorten only what is a label.

**Done means.** Every key changed is in a before/after table in the PR body; the checker covers each family you touched (extend the key families in `controlla_menu.py`, never loosen them); every e2e that matched an old string now matches the new one (select by role or `data-azione` where the text only finds the element); `python3 tools/controlla_i18n.py` and `python3 tools/verifica.py --changed` green.

**Out of scope.** Tooltips that are one sentence (≤ 90 characters, `tests/test_tooltip_corti.py`), `side.*` status messages, CLI help text, engine errors.

**Follow-ups.** Closing step of the brief: delete your row and this file.
