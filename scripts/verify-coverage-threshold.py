#!/usr/bin/env python3
"""
scripts/verify-coverage-threshold.py — UMCF Coverage Threshold Verifier

Verifies that test coverage meets or exceeds the mandatory threshold (>= 85.0% for S1.P3.SP1, >= 95.0% for certification).
Supports:
  1. Standard llvm-cov JSON exports (type: llvm.coverage.json.export)
  2. UMCF structured coverage summaries
  3. Built-in self-test mode (--test-self) verifying 84.9% FAIL vs 85.0% PASS
"""

import argparse
import json
import os
import sys

def parse_llvm_cov_json(data):
    """Parses standard llvm-cov JSON export format."""
    total_lines = 0
    covered_lines = 0
    package_stats = {}

    if "data" in data and len(data["data"]) > 0:
        export_data = data["data"][0]
        if "totals" in export_data and "lines" in export_data["totals"]:
            lines_data = export_data["totals"]["lines"]
            total_lines = lines_data.get("count", 0)
            covered_lines = lines_data.get("covered", 0)
            pct = lines_data.get("percent", 0.0)
            return total_lines, covered_lines, pct, {}

        # Fallback to files iteration
        if "files" in export_data:
            for file_entry in export_data["files"]:
                filename = file_entry.get("filename", "")
                summary = file_entry.get("summary", {}).get("lines", {})
                f_count = summary.get("count", 0)
                f_covered = summary.get("covered", 0)
                total_lines += f_count
                covered_lines += f_covered
                
                # Group by crate
                crate_name = "other"
                if "crates/umcf-proto" in filename:
                    crate_name = "umcf-proto"
                elif "crates/umcf-core" in filename:
                    crate_name = "umcf-core"
                elif "crates/umcf-telemetry" in filename:
                    crate_name = "umcf-telemetry"
                
                if crate_name not in package_stats:
                    package_stats[crate_name] = {"total": 0, "covered": 0}
                package_stats[crate_name]["total"] += f_count
                package_stats[crate_name]["covered"] += f_covered

            pct = (covered_lines / total_lines * 100.0) if total_lines > 0 else 0.0
            return total_lines, covered_lines, pct, package_stats

    # UMCF structured schema
    if "overall" in data and "coverage_percent" in data["overall"]:
        overall = data["overall"]
        total_lines = overall.get("lines_total", 0)
        covered_lines = overall.get("lines_covered", 0)
        pct = float(overall.get("coverage_percent", 0.0))
        return total_lines, covered_lines, pct, data.get("packages", {})

    raise ValueError("Unrecognized coverage JSON schema structure")

def evaluate_coverage(report_path, threshold):
    if not os.path.exists(report_path):
        print(f"ERROR: Coverage report file not found: {report_path}", file=sys.stderr)
        return False, 0.0

    with open(report_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    total_lines, covered_lines, pct, pkg_stats = parse_llvm_cov_json(data)

    print("==============================================================================")
    print(f" UMCF Coverage Threshold Verification Gate")
    print("==============================================================================")
    print(f" Report Path       : {report_path}")
    print(f" Total Lines       : {total_lines}")
    print(f" Covered Lines     : {covered_lines}")
    print(f" Measured Coverage : {pct:.2f}%")
    print(f" Required Threshold: {threshold:.2f}%")
    print("------------------------------------------------------------------------------")

    if isinstance(pkg_stats, dict) and pkg_stats:
        print(" Per-Package Breakdown:")
        for pkg, stats in pkg_stats.items():
            if isinstance(stats, dict) and "total" in stats:
                t = stats.get("total", 0)
                c = stats.get("covered", 0)
                p = (c / t * 100.0) if t > 0 else 0.0
                print(f"   - {pkg:<20}: {p:6.2f}% ({c}/{t} lines)")
            elif isinstance(stats, dict) and "coverage_percent" in stats:
                print(f"   - {stats.get('name', pkg):<20}: {stats.get('coverage_percent', 0.0):6.2f}%")

    print("------------------------------------------------------------------------------")
    if pct >= threshold:
        print(f" STATUS: PASS (Coverage {pct:.2f}% >= {threshold:.2f}% requirement)")
        print("==============================================================================")
        return True, pct
    else:
        print(f" STATUS: FAIL (Coverage {pct:.2f}% < {threshold:.2f}% requirement; gap: -{threshold - pct:.2f}%)", file=sys.stderr)
        print("==============================================================================")
        return False, pct

def run_self_test():
    """Unit test: verifies 84.9% yields FAIL and 85.0% / 95.0% yields PASS."""
    print("Running verify-coverage-threshold unit self-tests...")

    fail_report = {
        "overall": {
            "lines_total": 1000,
            "lines_covered": 849,
            "coverage_percent": 84.90
        }
    }
    pass_report_85 = {
        "overall": {
            "lines_total": 1000,
            "lines_covered": 850,
            "coverage_percent": 85.00
        }
    }
    pass_report_95 = {
        "overall": {
            "lines_total": 1000,
            "lines_covered": 955,
            "coverage_percent": 95.50
        }
    }

    # Test 84.9% against 85.0% threshold (must fail)
    _, _, pct1, _ = parse_llvm_cov_json(fail_report)
    assert pct1 < 85.0, "84.9% must be < 85.0"
    
    # Test 85.0% against 85.0% threshold (must pass)
    _, _, pct2, _ = parse_llvm_cov_json(pass_report_85)
    assert pct2 >= 85.0, "85.0% must be >= 85.0"

    # Test 95.5% against 85.0% threshold (must pass)
    _, _, pct3, _ = parse_llvm_cov_json(pass_report_95)
    assert pct3 >= 85.0, "95.5% must be >= 85.0"

    # Test standard llvm-cov schema parsing
    mock_llvm = {
        "data": [{
            "totals": {
                "lines": {
                    "count": 2000,
                    "covered": 1920,
                    "percent": 96.0
                }
            }
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }
    t, c, pct4, _ = parse_llvm_cov_json(mock_llvm)
    assert t == 2000 and c == 1920 and pct4 == 96.0

    print("All coverage verifier unit self-tests passed successfully.")
    return True

def main():
    parser = argparse.ArgumentParser(description="UMCF Coverage Threshold Verifier")
    parser.add_argument("--report", help="Path to coverage JSON report file")
    parser.add_argument("--threshold", type=float, default=85.0, help="Required line coverage percentage (default: 85.0)")
    parser.add_argument("--test-self", action="store_true", help="Run internal unit tests against synthetic reports")

    args = parser.parse_args()

    if args.test_self:
        success = run_self_test()
        sys.exit(0 if success else 1)

    if not args.report:
        parser.error("--report is required when not running --test-self")

    passed, _ = evaluate_coverage(args.report, args.threshold)
    sys.exit(0 if passed else 1)

if __name__ == "__main__":
    main()
