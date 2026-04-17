# Runtime Orchestrator Pool Console Release Checklist

## Scope

- Phase 1: unified Codex runtime shutdown, replacement cleanup, orphan sweep, runtime ledger
- Phase 2: launch restore decoupled from runtime acquisition, budgeted Hot/Warm/Cold pool
- Phase 3: Settings `Runtime Pool Console`, runtime snapshot/mutate commands, budget persistence

## Validation Matrix

- Startup restore:
  - visible workspaces restore thread metadata without bulk `connect_workspace`
  - active Codex workspace acquires runtime lazily on demand
- Runtime budget:
  - `codexMaxHotRuntimes` and `codexMaxWarmRuntimes` cap idle runtime count
  - `codexWarmTtlSeconds` expires idle runtimes back to cold
- Cleanup:
  - app exit drains managed Codex runtimes when `runtimeForceCleanupOnExit=true`
  - next launch orphan sweep clears stale ledger entries when `runtimeOrphanSweepOnLaunch=true`
- Console operations:
  - refresh snapshot
  - pin / unpin runtime
  - release runtime to cold
  - close runtime
  - persist pool settings through app settings

## Known Baseline Gaps Outside This Change

- `npm run typecheck` still fails on pre-existing issues in:
  - `src/features/composer/components/ChatInputBox/selectors/ModeSelect.tsx`
  - `src/features/messages/components/Messages.tsx`

## Rollout Notes

- Default behavior is now conservative: launch restores threads, not background Codex runtimes.
- Users can raise budgets in Settings if they prefer more warm capacity on faster machines.
- Diagnostics in the console are the first stop for investigating orphan cleanup and force-kill counts.
