# AGENTS.md: Repo hygiene

Scope: this file covers *repo hygiene*: branching, remotes, CI, cleanup. It is not project documentation.

This repo follows the branch/release workflow documented in `CONTRIBUTING.md`;
read and follow it for any git, branch, or release work here (the
single-trunk model, `feature/x`/`fix/x` branch naming, how RC tags work,
etc). Don't improvise a different workflow. The short version: there is one
long-lived branch, `main`; no `dev` or `beta` branch exists. `main`
auto-publishes a dev-track build on every push. "Beta" and "stable" are both
just tags, not branches: push a `vX.Y.Z-rc.N` tag to publish a beta-track
build, push a plain `vX.Y.Z` tag to cut the signed stable release.
"Freezing" for stabilization means pausing pushes to `main`, not moving a
branch. This replaced an earlier three-branch (`dev`/`beta`/`main`) model
after `main` was found to have silently rotted out of sync with `dev`/`beta`
across most repos in this ecosystem: a manual "merge beta into main
monthly" step nobody reliably did across a dozen-plus repos. Collapsing to
one branch removes the class of bug; there's nothing left that can fall out
of sync.

## Remotes
- `origin`: Forgejo (`git.breadway.dev`, SSH), authoritative.
- `github`: GitHub mirror. Push both when publishing.

## CI
- `check.yml`: clippy + test on `feature/**` and `fix/**` (fast-fail before
  a change reaches `main`).
- `dev-release.yml` triggers on `push: branches: ['main']`.
- `rc-release.yml` triggers on `push: tags: ['v*']` gated to *only* run for
  `-rc.` tags.
- `release.yml` triggers on `push: tags: ['v*']` gated to skip any tag
  containing `-rc.`; that's the signed stable release.
- No build/lint/test CI runs on ordinary commits or PRs to `main` beyond the
  dev-track workflow above. See bread-ecosystem's `docs/release-channels.md`
  for the full track (stable/beta/dev) policy.

## Don't
- Don't embed credentials in remote URLs; SSH or a credential helper only.
- Don't merge this repo with `bread-screenshots` (the crate in
  `bread-ecosystem`). Different jobs: breadshot is the user-facing
  grim/slurp/wl-copy orchestrator; `bread-screenshots` is the capture
  harness used by `bread-capture` to screenshot sibling apps in CI. They
  share a grim backend and nothing else.
