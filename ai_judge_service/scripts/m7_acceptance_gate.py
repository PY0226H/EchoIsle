#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app.m7_acceptance_gate import (
    GateThresholds,
    default_report_path,
    render_markdown_report,
    run_gate,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run AI Judge M7 acceptance gate.")
    parser.add_argument(
        "--test-module",
        action="append",
        dest="test_modules",
        default=[],
        help="unittest module or file path, repeatable. default: tests/test_m7_acceptance.py",
    )
    parser.add_argument(
        "--skip-tests",
        action="store_true",
        help="skip unittest modules",
    )
    parser.add_argument(
        "--skip-load",
        action="store_true",
        help="skip in-process dispatch load check",
    )
    parser.add_argument(
        "--python-exec",
        default=sys.executable,
        help="python executable used for unittest subprocess",
    )
    parser.add_argument("--load-total-requests", type=int, default=80)
    parser.add_argument("--load-concurrency", type=int, default=8)
    parser.add_argument("--min-success-rate", type=float, default=0.98)
    parser.add_argument("--max-p95-ms", type=float, default=5000.0)
    parser.add_argument(
        "--report-out",
        default=str(default_report_path()),
        help="markdown report output path",
    )
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    test_modules = args.test_modules or ["tests/test_m7_acceptance.py"]
    thresholds = GateThresholds(
        min_success_rate=args.min_success_rate,
        max_p95_latency_ms=args.max_p95_ms,
    )
    result = asyncio.run(
        run_gate(
            test_modules=test_modules,
            run_tests=not args.skip_tests,
            run_load=not args.skip_load,
            python_exec=args.python_exec,
            load_total_requests=args.load_total_requests,
            load_concurrency=args.load_concurrency,
            thresholds=thresholds,
        )
    )

    report_path = Path(args.report_out).resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(render_markdown_report(result), encoding="utf-8")

    print(f"[m7-acceptance-gate] result={'PASS' if result.passed else 'FAIL'}")
    print(f"[m7-acceptance-gate] report={report_path}")
    return 0 if result.passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
