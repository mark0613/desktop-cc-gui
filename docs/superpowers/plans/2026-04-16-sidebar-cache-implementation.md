# Sidebar Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist sidebar workspaces and per-workspace thread summaries so startup can render cached sidebar data immediately and then replace it with live results.

**Architecture:** Add a versioned sidebar snapshot helper in the thread storage layer, hydrate cached workspaces in `useWorkspaces`, hydrate cached thread summaries in `useThreads`, and rewrite the snapshot after successful live refreshes. Keep `Sidebar` presentation-only and preserve `preserveState: true` refresh semantics.

**Tech Stack:** React 19 hooks, Vitest, Testing Library, existing `clientStore` persistence helpers

---

### Task 1: Add failing tests for sidebar snapshot storage

**Files:**
- Create: `src/features/threads/utils/sidebarSnapshot.test.ts`
- Modify: `src/features/threads/utils/threadStorage.ts`
- Test: `src/features/threads/utils/sidebarSnapshot.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
it("loads a valid sidebar snapshot and rejects malformed data", () => {
  writeClientStoreValue("threads", "sidebarSnapshot", {
    version: 1,
    updatedAt: 123,
    workspaces: [
      {
        id: "ws-1",
        name: "repo",
        path: "/tmp/repo",
        connected: true,
        settings: { sidebarCollapsed: false },
      },
    ],
    threadsByWorkspace: {
      "ws-1": [{ id: "t-1", name: "Chat", updatedAt: 123 }],
    },
  });

  expect(loadSidebarSnapshot()).toEqual({
    version: 1,
    updatedAt: 123,
    workspaces: [
      expect.objectContaining({ id: "ws-1", name: "repo" }),
    ],
    threadsByWorkspace: {
      "ws-1": [expect.objectContaining({ id: "t-1", name: "Chat" })],
    },
  });

  writeClientStoreValue("threads", "sidebarSnapshot", { version: 1, bad: true });

  expect(loadSidebarSnapshot()).toBeNull();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/features/threads/utils/sidebarSnapshot.test.ts`
Expected: FAIL because `loadSidebarSnapshot` does not exist yet

- [ ] **Step 3: Write minimal implementation**

```ts
export type SidebarSnapshot = {
  version: 1;
  updatedAt: number;
  workspaces: WorkspaceInfo[];
  threadsByWorkspace: Record<string, ThreadSummary[]>;
};

export function loadSidebarSnapshot(): SidebarSnapshot | null {
  const raw = getClientStoreSync<unknown>("threads", "sidebarSnapshot");
  return normalizeSidebarSnapshot(raw);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/features/threads/utils/sidebarSnapshot.test.ts`
Expected: PASS

### Task 2: Add failing tests for cached workspace hydration

**Files:**
- Modify: `src/features/workspaces/hooks/useWorkspaces.test.tsx`
- Modify: `src/features/workspaces/hooks/useWorkspaces.ts`
- Test: `src/features/workspaces/hooks/useWorkspaces.test.tsx`

- [ ] **Step 1: Write the failing test**

```ts
it("hydrates cached workspaces before live refresh resolves", async () => {
  writeClientStoreValue("threads", "sidebarSnapshot", {
    version: 1,
    updatedAt: 123,
    workspaces: [workspaceOne],
    threadsByWorkspace: {},
  });

  let resolveList: (value: WorkspaceInfo[]) => void = () => {};
  vi.mocked(listWorkspaces).mockReturnValue(
    new Promise((resolve) => {
      resolveList = resolve;
    }),
  );

  const { result } = renderHook(() => useWorkspaces());

  expect(result.current.workspaces).toEqual([workspaceOne]);

  await act(async () => {
    resolveList([workspaceTwo]);
  });

  await waitFor(() => {
    expect(result.current.workspaces).toEqual([workspaceTwo]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/features/workspaces/hooks/useWorkspaces.test.tsx`
Expected: FAIL because cached workspaces are not used yet

- [ ] **Step 3: Write minimal implementation**

```ts
const initialSidebarSnapshot = loadSidebarSnapshot();
const [workspaces, setWorkspaces] = useState<WorkspaceInfo[]>(
  () => initialSidebarSnapshot?.workspaces ?? [],
);
const [hasLoaded, setHasLoaded] = useState(
  () => (initialSidebarSnapshot?.workspaces.length ?? 0) > 0,
);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/features/workspaces/hooks/useWorkspaces.test.tsx`
Expected: PASS

### Task 3: Add failing tests for cached thread-summary hydration

**Files:**
- Create: `src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`
- Modify: `src/features/threads/hooks/useThreads.ts`
- Modify: `src/features/threads/hooks/useThreadsReducer.ts`
- Test: `src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`

- [ ] **Step 1: Write the failing test**

```ts
it("hydrates cached thread summaries before live thread list resolves", () => {
  writeClientStoreValue("threads", "sidebarSnapshot", {
    version: 1,
    updatedAt: 123,
    workspaces: [workspace],
    threadsByWorkspace: {
      "ws-1": [{ id: "thread-1", name: "Cached chat", updatedAt: 123 }],
    },
  });

  const { result } = renderHook(() =>
    useThreads({
      activeWorkspace: workspace,
      onWorkspaceConnected: vi.fn(),
    }),
  );

  expect(result.current.threadsByWorkspace["ws-1"]).toEqual([
    expect.objectContaining({ id: "thread-1", name: "Cached chat" }),
  ]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`
Expected: FAIL because initial reducer state is empty

- [ ] **Step 3: Write minimal implementation**

```ts
const [state, dispatch] = useReducer(
  threadReducer,
  undefined,
  createInitialThreadStateFromSidebarSnapshot,
);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`
Expected: PASS

### Task 4: Persist fresh live results back into the snapshot

**Files:**
- Modify: `src/features/workspaces/hooks/useWorkspaces.ts`
- Modify: `src/features/threads/hooks/useThreadActions.ts`
- Modify: `src/features/threads/utils/threadStorage.ts`
- Test: `src/features/workspaces/hooks/useWorkspaces.test.tsx`
- Test: `src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`

- [ ] **Step 1: Write the failing test**

```ts
it("rewrites cached thread summaries after a successful live list", async () => {
  vi.mocked(listThreads).mockResolvedValue({
    result: {
      data: [{ id: "thread-2", preview: "Fresh chat", updated_at: 456 }],
      nextCursor: null,
    },
  });

  const { result } = renderHook(() =>
    useThreads({
      activeWorkspace: workspace,
      onWorkspaceConnected: vi.fn(),
    }),
  );

  await act(async () => {
    await result.current.listThreadsForWorkspace(workspace);
  });

  expect(loadSidebarSnapshot()?.threadsByWorkspace["ws-1"]).toEqual([
    expect.objectContaining({ id: "thread-2" }),
  ]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/features/threads/hooks/useThreads.sidebar-cache.test.tsx src/features/workspaces/hooks/useWorkspaces.test.tsx`
Expected: FAIL because live success does not update the snapshot yet

- [ ] **Step 3: Write minimal implementation**

```ts
saveSidebarSnapshotThreads(workspace.id, nextThreads);
saveSidebarSnapshotWorkspaces(entries);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/features/threads/hooks/useThreads.sidebar-cache.test.tsx src/features/workspaces/hooks/useWorkspaces.test.tsx`
Expected: PASS

### Task 5: Verify the complete change set

**Files:**
- Modify: `AGENTS.md` final report only

- [ ] **Step 1: Run focused tests**

Run: `npx vitest run src/features/threads/utils/sidebarSnapshot.test.ts src/features/workspaces/hooks/useWorkspaces.test.tsx src/features/threads/hooks/useThreads.sidebar-cache.test.tsx`
Expected: PASS

- [ ] **Step 2: Run broader safety checks**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 3: Run lint**

Run: `npm run lint`
Expected: PASS
