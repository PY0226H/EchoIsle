import unittest

from app.m7_acceptance_gate import (
    GateLoadResult,
    GateThresholds,
    evaluate_load_gate,
    percentile,
    run_inprocess_dispatch_load,
)


class M7AcceptanceGateTests(unittest.IsolatedAsyncioTestCase):
    def test_percentile_should_handle_empty_and_boundaries(self) -> None:
        self.assertEqual(percentile([], 95), 0.0)
        self.assertEqual(percentile([5, 1, 9], 0), 1.0)
        self.assertEqual(percentile([5, 1, 9], 100), 9.0)

    async def test_run_inprocess_dispatch_load_should_return_metrics(self) -> None:
        result = await run_inprocess_dispatch_load(total_requests=6, concurrency=2)
        self.assertEqual(result.total_requests, 6)
        self.assertEqual(result.succeeded, 6)
        self.assertEqual(result.failed, 0)
        self.assertEqual(result.success_rate, 1.0)
        self.assertGreaterEqual(result.p95_latency_ms, 0.0)

    def test_evaluate_load_gate_should_fail_on_threshold_violation(self) -> None:
        thresholds = GateThresholds(min_success_rate=0.99, max_p95_latency_ms=10.0)
        load = GateLoadResult(
            total_requests=10,
            succeeded=9,
            failed=1,
            success_rate=0.9,
            p50_latency_ms=6.0,
            p95_latency_ms=12.0,
            p99_latency_ms=15.0,
            max_latency_ms=18.0,
        )
        passed, reasons = evaluate_load_gate(load, thresholds)
        self.assertFalse(passed)
        self.assertEqual(len(reasons), 2)
        self.assertIn("success_rate", reasons[0])
        self.assertIn("p95_latency_ms", reasons[1])


if __name__ == "__main__":
    unittest.main()
