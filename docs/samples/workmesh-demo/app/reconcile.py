import argparse
import json
from typing import Dict, List, Tuple


def reconcile(remote: Dict[str, int], local: Dict[str, int]) -> dict:
    remote_keys = set(remote.keys())
    local_keys = set(local.keys())

    missing_remote = sorted(local_keys - remote_keys)
    missing_local = sorted(remote_keys - local_keys)
    mismatched: List[Tuple[str, int, int]] = []

    for sku in sorted(remote_keys & local_keys):
        if remote[sku] != local[sku]:
            mismatched.append((sku, remote[sku], local[sku]))

    return {
        "totals": {
            "remote": len(remote_keys),
            "local": len(local_keys),
            "mismatched": len(mismatched),
            "missing_remote": len(missing_remote),
            "missing_local": len(missing_local),
        },
        "missing_remote": missing_remote,
        "missing_local": missing_local,
        "mismatched": [
            {"sku": sku, "remote_qty": remote_qty, "local_qty": local_qty}
            for sku, remote_qty, local_qty in mismatched
        ],
    }


def load_json(path: str) -> Dict[str, int]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("remote")
    parser.add_argument("local")
    parser.add_argument("--format", choices=["json", "csv"], default="json")
    args = parser.parse_args()

    report = reconcile(load_json(args.remote), load_json(args.local))
    if args.format == "json":
        print(json.dumps(report, indent=2))
    else:
        print("sku,remote_qty,local_qty")
        for row in report["mismatched"]:
            print(f"{row['sku']},{row['remote_qty']},{row['local_qty']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
