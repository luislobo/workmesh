import os
import sys

sys.path.append(os.path.dirname(os.path.dirname(__file__)))

from adapter import process_payload


def test_apply_create():
    snapshot = {}
    payload = {
        "source_id": "vendor-1",
        "version": 1,
        "event_type": "create",
        "items": [{"sku": "ABC", "qty": 5}],
    }
    seen = set()
    updated, applied, errors = process_payload(snapshot, payload, seen)
    assert applied is True
    assert errors == []
    assert updated["ABC"] == 5


def test_idempotent_duplicate():
    snapshot = {}
    payload = {
        "source_id": "vendor-1",
        "version": 1,
        "event_type": "update",
        "items": [{"sku": "ABC", "qty": 7}],
    }
    seen = set()
    process_payload(snapshot, payload, seen)
    updated, applied, errors = process_payload(snapshot, payload, seen)
    assert applied is False
    assert "duplicate payload" in errors[0]


def test_validation_failure():
    snapshot = {}
    payload = {"source_id": "vendor-1", "version": "bad"}
    updated, applied, errors = process_payload(snapshot, payload, set())
    assert applied is False
    assert errors
