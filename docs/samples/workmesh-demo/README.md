# WorkMesh Demo: Inventory Sync

This sample project demonstrates WorkMesh features using a small “Inventory Sync” scenario.
It includes tasks with dependencies, a repo-local context, and a Truth Ledger record.

## Layout
- `docs/projects/wm-demo/`: PRD/decisions/updates scaffolding.
- `tasks/`: tasks with dependencies and rich metadata.
- `.workmesh/`: repo-local context, truth, and derived state.
- `workmesh/context.json`: current objective and task scope.
- `workmesh/truth/`: decision records.
- `app/`: minimal reference implementation + sample payloads.

## Quick walkthrough
From the WorkMesh repo root:

1. List tasks
   ```bash
   workmesh --root docs/samples/workmesh-demo list
   ```

2. Inspect a task
   ```bash
   workmesh --root docs/samples/workmesh-demo show task-isnv-002
   ```

3. Show current context
   ```bash
   workmesh --root docs/samples/workmesh-demo context show
   ```

4. List truth records
   ```bash
   workmesh --root docs/samples/workmesh-demo truth list
   ```

5. Get next actionable task
   ```bash
   workmesh --root docs/samples/workmesh-demo next
   ```

6. Run the sample adapter + reconciliation
   ```bash
   python docs/samples/workmesh-demo/app/adapter.py docs/samples/workmesh-demo/app/samples/payload.json docs/samples/workmesh-demo/app/samples/snapshot.json
   python docs/samples/workmesh-demo/app/reconcile.py docs/samples/workmesh-demo/app/samples/remote.json docs/samples/workmesh-demo/app/samples/snapshot.json --format json
   ```

## Example workflow highlights
- Tasks include `Description`, `Acceptance Criteria`, and outcome-focused `Definition of Done`.
- Dependencies model sequencing: contract -> adapter -> reconciliation.
- Truth Ledger captures durable decisions for the feature.
- `task-isnv-005` is marked `Done` and ready to archive if you want to demo cleanup.
