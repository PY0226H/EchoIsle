import unittest
from types import SimpleNamespace

from fastapi import HTTPException

from app.dispatch_controller import DispatchRuntimeConfig, process_dispatch_request


class _FakeReport:
    def __init__(self, *, winner: str = "pro", needs_draw_vote: bool = False, provider: str = "openai") -> None:
        self.winner = winner
        self.needs_draw_vote = needs_draw_vote
        self.payload = {"provider": provider}

    def model_dump(self, *, mode: str = "python") -> dict:
        return {"winner": self.winner, "needsDrawVote": self.needs_draw_vote, "mode": mode}


def _build_request(messages: list[object] | None = None) -> SimpleNamespace:
    return SimpleNamespace(
        job=SimpleNamespace(
            job_id=42,
            style_mode="rational",
        ),
        messages=messages if messages is not None else [SimpleNamespace(message_id=1)],
    )


class DispatchControllerTests(unittest.IsolatedAsyncioTestCase):
    async def test_process_dispatch_request_should_mark_failed_when_messages_empty(self) -> None:
        request = _build_request(messages=[])
        failed_calls: list[tuple[int, str]] = []

        async def build_report_by_runtime(*_args: object, **_kwargs: object) -> _FakeReport:
            raise AssertionError("should not build report when no messages")

        async def callback_failed(job_id: int, error_message: str) -> None:
            failed_calls.append((job_id, error_message))

        async def callback_report(*_args: object, **_kwargs: object) -> None:
            raise AssertionError("should not call callback_report on empty messages")

        result = await process_dispatch_request(
            request=request,
            runtime_cfg=DispatchRuntimeConfig(process_delay_ms=0, judge_style_mode="rational"),
            build_report_by_runtime=build_report_by_runtime,
            callback_report=callback_report,
            callback_failed=callback_failed,
        )

        self.assertEqual(result["status"], "marked_failed")
        self.assertEqual(failed_calls, [(42, "empty debate messages, cannot judge")])

    async def test_process_dispatch_request_should_mark_failed_when_runtime_error(self) -> None:
        request = _build_request()
        failed_calls: list[tuple[int, str]] = []

        async def build_report_by_runtime(*_args: object, **_kwargs: object) -> _FakeReport:
            raise RuntimeError("runtime exploded")

        async def callback_failed(job_id: int, error_message: str) -> None:
            failed_calls.append((job_id, error_message))

        async def callback_report(*_args: object, **_kwargs: object) -> None:
            raise AssertionError("should not call callback_report after runtime error")

        result = await process_dispatch_request(
            request=request,
            runtime_cfg=DispatchRuntimeConfig(process_delay_ms=0, judge_style_mode="rational"),
            build_report_by_runtime=build_report_by_runtime,
            callback_report=callback_report,
            callback_failed=callback_failed,
        )

        self.assertEqual(result["status"], "marked_failed")
        self.assertEqual(len(failed_calls), 1)
        self.assertEqual(failed_calls[0][0], 42)
        self.assertIn("judge runtime failed", failed_calls[0][1])

    async def test_process_dispatch_request_should_raise_when_runtime_error_and_callback_failed_error(self) -> None:
        request = _build_request()

        async def build_report_by_runtime(*_args: object, **_kwargs: object) -> _FakeReport:
            raise RuntimeError("runtime exploded")

        async def callback_failed(*_args: object, **_kwargs: object) -> None:
            raise RuntimeError("callback failed also exploded")

        async def callback_report(*_args: object, **_kwargs: object) -> None:
            raise AssertionError("should not call callback_report after runtime error")

        with self.assertRaises(HTTPException) as ctx:
            await process_dispatch_request(
                request=request,
                runtime_cfg=DispatchRuntimeConfig(process_delay_ms=0, judge_style_mode="rational"),
                build_report_by_runtime=build_report_by_runtime,
                callback_report=callback_report,
                callback_failed=callback_failed,
            )

        self.assertEqual(ctx.exception.status_code, 502)
        self.assertIn("runtime failed and callback_failed failed", str(ctx.exception.detail))

    async def test_process_dispatch_request_should_raise_when_callback_report_error(self) -> None:
        request = _build_request()

        async def build_report_by_runtime(*_args: object, **_kwargs: object) -> _FakeReport:
            return _FakeReport()

        async def callback_failed(*_args: object, **_kwargs: object) -> None:
            raise AssertionError("should not call callback_failed on success path")

        async def callback_report(*_args: object, **_kwargs: object) -> None:
            raise RuntimeError("callback report failed")

        with self.assertRaises(HTTPException) as ctx:
            await process_dispatch_request(
                request=request,
                runtime_cfg=DispatchRuntimeConfig(process_delay_ms=0, judge_style_mode="rational"),
                build_report_by_runtime=build_report_by_runtime,
                callback_report=callback_report,
                callback_failed=callback_failed,
            )

        self.assertEqual(ctx.exception.status_code, 502)
        self.assertIn("callback report failed", str(ctx.exception.detail))

    async def test_process_dispatch_request_should_return_summary_when_success(self) -> None:
        request = _build_request()
        callback_payloads: list[tuple[int, dict]] = []
        sleep_calls: list[float] = []

        async def build_report_by_runtime(
            req: SimpleNamespace,
            effective_style_mode: str,
            style_mode_source: str,
        ) -> _FakeReport:
            self.assertEqual(req.job.job_id, 42)
            self.assertEqual(effective_style_mode, "rational")
            self.assertEqual(style_mode_source, "system_config")
            return _FakeReport(winner="con", needs_draw_vote=True, provider="openai")

        async def callback_report(job_id: int, payload: dict) -> None:
            callback_payloads.append((job_id, payload))

        async def callback_failed(*_args: object, **_kwargs: object) -> None:
            raise AssertionError("should not call callback_failed on success path")

        async def fake_sleep(seconds: float) -> None:
            sleep_calls.append(seconds)

        result = await process_dispatch_request(
            request=request,
            runtime_cfg=DispatchRuntimeConfig(process_delay_ms=250, judge_style_mode="rational"),
            build_report_by_runtime=build_report_by_runtime,
            callback_report=callback_report,
            callback_failed=callback_failed,
            sleep_fn=fake_sleep,
        )

        self.assertEqual(sleep_calls, [0.25])
        self.assertEqual(callback_payloads[0][0], 42)
        self.assertEqual(result["accepted"], True)
        self.assertEqual(result["winner"], "con")
        self.assertEqual(result["needsDrawVote"], True)
        self.assertEqual(result["provider"], "openai")


if __name__ == "__main__":
    unittest.main()
