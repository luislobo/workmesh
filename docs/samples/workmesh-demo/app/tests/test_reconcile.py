import os
import sys

sys.path.append(os.path.dirname(os.path.dirname(__file__)))

from reconcile import reconcile


def test_reconcile_reports_mismatch_and_missing():
    remote = {"A": 2, "B": 1}
    local = {"A": 3, "C": 5}
    report = reconcile(remote, local)

    assert report["totals"]["remote"] == 2
    assert report["totals"]["local"] == 2
    assert report["totals"]["mismatched"] == 1
    assert report["missing_remote"] == ["C"]
    assert report["missing_local"] == ["B"]
    assert report["mismatched"][0]["sku"] == "A"
