# Contributing to mzLab

mzLab is a professional free LC-MS analysis program (browser site plus desktop and CLI app). The project is maintained on a best-effort basis: reports and proposals are welcome, with no guaranteed response time.

## Reporting
- Errors and questions: [new issue](https://github.com/Freeesta/mzlab/issues/new/choose). **Never attach laboratory data or personal data**: the repository is public.
- Vulnerabilities: not in a public issue, see [SECURITY.md](SECURITY.md).
- Guides for people who use the program: [docs/utenti/](docs/utenti/README.md).

## Proposing a change
1. Open an issue first to describe the change, above all if it changes the behaviour or the interface.
2. Work on a branch `claude/<topic>` or `<your-name>/<topic>` starting from `origin/main` (agent chats always use `claude/<topic>`); one pull request per topic. Merging is done with a *merge commit* (no rebase, no force-push) after a green CI.
3. The rules of the project are in [CLAUDE.md](CLAUDE.md) and in [docs/agenti/](docs/agenti/): the user decides (evidence shown, every automatic choice logged), lightness (no bundler), English-only interface through the catalogs in `mzlab/web/lang/`, accessibility (WCAG 2.2 AA, every chart with its data table), architecture and tests.
4. Before pushing: `python3 tools/pre_pr.py` (fast; `--full` forces every test). For changed browser tests only: `python3 tools/verifica.py --changed` (see `docs/agenti/test.md`).

## Data and content
No laboratory data or personal names go in the repository: they live in a private repository. The public examples are anonymous.

## Licence
With the pull request you agree that your contribution is released under the project's MIT licence ([LICENSE](LICENSE)).
