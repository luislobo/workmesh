# Inventory Sync Demo App

Minimal reference implementation for the WorkMesh demo project. This code is intentionally small
and focuses on clarity over completeness.

## Files
- `contract.schema.json`: payload contract (required fields + basic structure).
- `adapter.py`: validate + apply create/update/delete events to a snapshot.
- `reconcile.py`: compare remote vs. local snapshots and emit a report.
- `tests/`: basic unit tests.

## Quick run
```bash
python adapter.py samples/payload.json samples/snapshot.json > samples/snapshot.updated.json
python reconcile.py samples/remote.json samples/snapshot.updated.json --format json
```

## Notes
- Idempotency is keyed by `(source_id, version)`.
- Validation is minimal and intended for demonstration.
