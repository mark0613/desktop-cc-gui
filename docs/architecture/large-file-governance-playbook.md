# Large File Governance Playbook

## Scope

This playbook governs source files larger than 3000 lines in this repository.

- Threshold: `> 3000` lines
- Scanner: `scripts/check-large-files.mjs`
- Baseline report: `docs/architecture/large-file-baseline.md`

## Quality Gate

### Local checks

```bash
npm run check:large-files:baseline
npm run check:large-files:gate
```

### CI checks

- Workflow: `.github/workflows/large-file-governance.yml`
- Hard gate command: `npm run check:large-files:gate`
- Rule: any new `>3000` file fails PR checks.

## Capability Retention Matrix

| Area | Before | After | Retention Proof |
|---|---|---|---|
| Git history panel | Monolithic panel implementation | Split into modular panel/hooks/utils files | `src/features/git-history/components/GitHistoryPanel.tsx` + `git-history-panel/**` |
| App shell bootstrap | `App.tsx` monolith | `App.tsx` entry + `router/bootstrap/app-shell` split | `src/App.tsx`, `src/router.tsx`, `src/bootstrap.ts`, `src/app-shell.tsx` |
| Settings feature | `SettingsView.tsx` monolith | Sections/hooks/actions split | `src/features/settings/components/settings-view/**` |
| Rust backend bridge | `app_server.rs`, `engine/commands.rs`, `git/mod.rs` monoliths | command/service/helper modules extracted | `src-tauri/src/backend/*`, `src-tauri/src/engine/*`, `src-tauri/src/git/*` |
| CSS and i18n | Large single files | Split by parts with stable aggregator imports | `src/styles/*.part*.css`, `src/i18n/locales/*.part*.ts` |

## Rollback Manual

### Trigger

Rollback is required when any of the following occurs after modularization:

- App startup/navigation regression.
- SpecHub / GitHistory / Settings critical interaction breakage.
- Rust bridge command dispatch regression.
- CI hard gate false-positive due scanner bug.

### Fast rollback (single PR)

1. Revert modularization commit(s) for impacted area only.
2. Keep unrelated areas untouched.
3. Re-run:

```bash
npm run check:large-files:baseline
cargo check --manifest-path src-tauri/Cargo.toml
```

4. Open a follow-up hotfix issue with root cause and corrected split plan.

### Surgical rollback (partial)

1. Restore previous entry file as adapter layer, keep new modules in place.
2. Re-export legacy API from adapter to preserve call sites.
3. Add temporary feature flag or fallback path for unstable new branch.
4. Re-run target module tests and smoke checks.

## Merge Guardrails

- Do not use whole-file `--ours/--theirs` on high-risk files.
- Resolve conflicts semantically at state/action/render granularity.
- Verify key symbols still exist with `rg`.
- Keep PR notes with explicit “retained capability list”.

## Operational Notes

- Prefer incremental split by `state/actions/render`.
- Keep external API and exported types stable during split.
- If a file is still above 3000 after one split round, continue decomposition in the same PR chain.
