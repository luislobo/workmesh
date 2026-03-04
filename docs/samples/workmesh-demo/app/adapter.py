import json
import sys
from typing import Dict, List, Tuple


def validate_payload(payload: dict) -> List[str]:
    errors: List[str] = []
    for field in ("source_id", "version", "event_type", "items"):
        if field not in payload:
            errors.append(f"missing field: {field}")
    if "version" in payload and not isinstance(payload["version"], int):
        errors.append("version must be an integer")
    if payload.get("event_type") not in ("create", "update", "delete"):
        errors.append("event_type must be create|update|delete")
    items = payload.get("items")
    if items is None or not isinstance(items, list):
        errors.append("items must be a list")
    else:
        for idx, item in enumerate(items):
            if not isinstance(item, dict):
                errors.append(f"items[{idx}] must be an object")
                continue
            if not item.get("sku"):
                errors.append(f"items[{idx}].sku is required")
            if "qty" in item and not isinstance(item["qty"], int):
                errors.append(f"items[{idx}].qty must be an integer")
    return errors


def apply_event(snapshot: Dict[str, int], payload: dict) -> None:
    event_type = payload["event_type"]
    for item in payload["items"]:
        sku = item["sku"]
        if event_type == "delete":
            snapshot.pop(sku, None)
            continue
        qty = item.get("qty", 0)
        snapshot[sku] = qty


def process_payload(snapshot: Dict[str, int], payload: dict, seen_versions: set) -> Tuple[Dict[str, int], bool, List[str]]:
    errors = validate_payload(payload)
    if errors:
        return snapshot, False, errors

    idempotency_key = (payload["source_id"], payload["version"])
    if idempotency_key in seen_versions:
        return snapshot, False, ["duplicate payload (idempotent)"]

    apply_event(snapshot, payload)
    seen_versions.add(idempotency_key)
    return snapshot, True, []


def load_json(path: str) -> dict:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def main(argv: List[str]) -> int:
    if len(argv) < 3:
        print("usage: python adapter.py <payload.json> <snapshot.json>", file=sys.stderr)
        return 1

    payload = load_json(argv[1])
    snapshot = load_json(argv[2])
    seen_versions = set()

    snapshot, applied, errors = process_payload(snapshot, payload, seen_versions)
    if errors:
        print(json.dumps({"applied": applied, "errors": errors}, indent=2), file=sys.stderr)
        return 2

    print(json.dumps(snapshot, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
