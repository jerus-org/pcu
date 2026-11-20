# CLAUDE.md

Rules for Claude Code working in this repository.

## Git and pull requests

- Never commit to `main`. Branch from a freshly fetched `origin/main`, named `<type>/<description>` (`feat`, `fix`, `docs`, `chore`, `refactor`). A cloud session works on the branch it was given.
- Never merge, approve or close a pull request, including your own superseded ones: open it, return the URL, comment, and recommend closure. `#310` is a permanent test PR; never close it.
- Stop and report before any commit, amend or push, and wait for "go", "commit and push" or "commit and pr". After a review or fixes, apply them locally, then stop.
- Stage explicit paths; never `git add -A`.
- Before pushing to a branch that has a PR, check the PR's state: it may have been merged. A `[new branch]` line on push means a merged branch was re-created.
- One PR at a time per workpackage: wait for the merge before starting the next unless told otherwise.
- Never force-push a Renovate branch. A fix needed only because of a Renovate update goes on that PR's branch as a new commit.
- Prefer merge commits to squash, to keep attribution. Adopting a fork PR is the exception: see `docs/ADOPTING_FORK_PRS.md`.

## Commits

- Conventional Commits, subject under 50 characters, `git commit -s` for the sign-off. Commits are also GPG-signed; see `.claude/hooks/README.md` for the agent identity.
- A commit made with Claude Code carries these trailers before the sign-off:

  ```
  Generated-by: claude-code/<version> (model: <model-id>; operator: <operator-email>)
  Claude-Session: <session-url>
  ```

  `<version>` is `claude --version`, `<model-id>` the model serving the session, `<operator-email>` the person directing it.
- Never write the literal CI-skip token in a commit or PR message.

## Rust work

- Red/green TDD for all Rust work, features, fixes and refactors: write the failing test, show it fail, then make it pass. Write characterisation tests before refactoring and mutation-check them.
- Put `#[cfg(test)]` modules at the end of the file with the existing ones. Tests that spawn git or gpg run with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, give fixtures their own identity, and use an ephemeral `GNUPGHOME`.
- Before a PR, on every Rust PR even config-only ones, run what CI runs (read `.circleci/config.yml`): `cargo fmt --all -- --check` (nightly fmt first with `rustfmt-nightly.toml`; revert drift onto unrelated files), `cargo clippy --all --tests --all-features -- -D warnings`, `cargo nextest run --workspace --all-features`, `cargo test --doc`, `cargo audit`, `cargo msrv verify`, and the nightly doc build when a doc comment changes. Re-run the format check after any edit. A new advisory can fail a PR that touches no Rust.
- Run `/code-review` once per PR, on the final diff, before the commit. Report the findings and stop. Comment-only or docs-only PRs skip it.
- `rust-version` is the genuine minimum; do not contort code to protect a low MSRV.

## Working practice

- Fix the root cause, not the symptom. Age is not evidence of soundness.
- File a follow-up as an issue as soon as it is found, labelled `investigate`, with the evidence, why it was deferred, the options and a recommendation. One item per issue. Security findings are the exception: see the next rule.
- Report a security finding (a way around a check, an exposed or over-broad credential, a gap in verification) privately, never in a public issue, PR text, comment, doc or commit message. Say nothing public about it, not even that one exists: a placeholder or "moved to a private report" note also draws attention. If unsure whether something is a security finding, treat it as one.
- Private routes, in order: the repository's private vulnerability reporting (Security tab, "Report a vulnerability"); when the session cannot reach it (the session proxy blocks writes to that API), write a report file (title, severity, description), send it to the user in the same turn with the file-delivery tool, and log it as an issue in the private findings repository `jerus-org/security-findings` (attach it with `add_repo`, push access), assigned to `jerusdp`. That repository is for trusted agents only; `SECURITY.md` never mentions it. Highlight the finding strongly in the chat until the user has filed it.
- Show a CI gate working, not just an explanation: reintroduce the failure, watch it fail, then revert.
- Comments state only the rule a reader cannot infer from the code, and describe current behaviour only. History belongs in commits, PRs and issues.
- Published text (PRs, issues, docs) uses placeholders, not real bot or user identities. Policy files that need real identifiers are the exception. Never print a token or key.
- Use the Context7 connector (`resolve-library-id`, `query-docs`) for library documentation when it is available.
- pcu is an internal CI tool and may assume its environment: not being a faithful reimplementation of git or gpg is not a defect.
