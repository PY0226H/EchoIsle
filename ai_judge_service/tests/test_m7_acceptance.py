import unittest
from datetime import datetime, timezone

from app.app_factory import create_app, create_runtime
from app.models import (
    DispatchJob,
    DispatchMessage,
    DispatchSession,
    DispatchTopic,
    JudgeDispatchRequest,
)
from app.settings import Settings


class _FakeReport:
    def __init__(self) -> None:
        self.winner = "pro"
        self.needs_draw_vote = False
        self.payload = {
            "provider": "openai",
            "evidenceRefs": [{"messageId": 1, "reason": "test"}],
            "judgeAudit": {
                "promptHash": "hash-1",
                "model": "gpt-4.1-mini",
                "rubricVersion": "v1",
                "retrievalSnapshot": [],
                "degradationLevel": 0,
            },
        }
        self.rationale = "test rationale with enough chars"

    def model_dump(self, *, mode: str = "python") -> dict:
        return {
            "winner": self.winner,
            "needsDrawVote": self.needs_draw_vote,
            "rationale": self.rationale,
            "payload": self.payload,
            "stage_summaries": [
                {
                    "stage_no": 1,
                    "from_message_id": 1,
                    "to_message_id": 1,
                    "pro_score": 30,
                    "con_score": 28,
                    "summary": {"stageFocus": "opening"},
                }
            ],
            "mode": mode,
        }


def _build_settings(**overrides: object) -> Settings:
    base = {
        "ai_internal_key": "k",
        "chat_server_base_url": "http://chat",
        "report_path_template": "/r/{job_id}",
        "failed_path_template": "/f/{job_id}",
        "callback_timeout_secs": 8.0,
        "process_delay_ms": 0,
        "judge_style_mode": "rational",
        "provider": "mock",
        "openai_api_key": "",
        "openai_model": "gpt-4.1-mini",
        "openai_base_url": "https://api.openai.com/v1",
        "openai_timeout_secs": 25.0,
        "openai_temperature": 0.1,
        "openai_max_retries": 2,
        "openai_fallback_to_mock": True,
        "rag_enabled": True,
        "rag_knowledge_file": "",
        "rag_max_snippets": 4,
        "rag_max_chars_per_snippet": 280,
        "rag_query_message_limit": 80,
        "rag_source_whitelist": ("https://teamfighttactics.leagueoflegends.com/en-us/news",),
        "rag_backend": "file",
        "rag_openai_embedding_model": "text-embedding-3-small",
        "rag_milvus_uri": "",
        "rag_milvus_token": "",
        "rag_milvus_db_name": "",
        "rag_milvus_collection": "",
        "rag_milvus_vector_field": "embedding",
        "rag_milvus_content_field": "content",
        "rag_milvus_title_field": "title",
        "rag_milvus_source_url_field": "source_url",
        "rag_milvus_chunk_id_field": "chunk_id",
        "rag_milvus_tags_field": "tags",
        "rag_milvus_metric_type": "COSINE",
        "rag_milvus_search_limit": 20,
        "stage_agent_max_chunks": 12,
        "graph_v2_enabled": True,
        "reflection_enabled": True,
        "topic_memory_enabled": True,
        "rag_hybrid_enabled": True,
        "rag_rerank_enabled": True,
        "reflection_policy": "winner_mismatch_only",
        "reflection_low_margin_threshold": 3,
        "fault_injection_nodes": (),
        "degrade_max_level": 3,
        "trace_ttl_secs": 86400,
        "idempotency_ttl_secs": 86400,
        "redis_enabled": False,
        "redis_required": False,
        "redis_url": "redis://127.0.0.1:6379/0",
        "redis_pool_size": 20,
        "redis_key_prefix": "ai_judge:v2",
        "topic_memory_limit": 5,
        "topic_memory_min_evidence_refs": 1,
        "topic_memory_min_rationale_chars": 20,
        "topic_memory_min_quality_score": 0.55,
    }
    base.update(overrides)
    return Settings(**base)


def _build_request() -> JudgeDispatchRequest:
    now = datetime.now(timezone.utc)
    return JudgeDispatchRequest(
        job=DispatchJob(
            job_id=1,
            ws_id=1,
            session_id=2,
            requested_by=1,
            style_mode="rational",
            rejudge_triggered=False,
            requested_at=now,
        ),
        session=DispatchSession(
            status="judging",
            scheduled_start_at=now,
            actual_start_at=now,
            end_at=now,
        ),
        topic=DispatchTopic(
            title="test",
            description="desc",
            category="game",
            stance_pro="pro",
            stance_con="con",
            context_seed=None,
        ),
        messages=[
            DispatchMessage(
                message_id=1,
                speaker_tag="pro_1",
                user_id=None,
                side="pro",
                content="hello",
                created_at=now,
            )
        ],
        message_window_size=100,
        rubric_version="v1",
    )


class M7AcceptanceTests(unittest.IsolatedAsyncioTestCase):
    async def test_m7_happy_path_should_keep_dispatch_trace_and_replay_consistent(self) -> None:
        settings = _build_settings(ai_internal_key="m7-k1", provider="mock")
        callback_report_calls: list[tuple[int, dict]] = []
        callback_failed_calls: list[tuple[int, str]] = []

        async def fake_callback_report(*, cfg: object, job_id: int, payload: dict) -> None:
            callback_report_calls.append((job_id, payload))

        async def fake_callback_failed(*, cfg: object, job_id: int, error_message: str) -> None:
            callback_failed_calls.append((job_id, error_message))

        runtime = create_runtime(
            settings=settings,
            callback_report_impl=fake_callback_report,
            callback_failed_impl=fake_callback_failed,
        )
        app = create_app(runtime)
        request = _build_request()

        dispatch_route = next(route for route in app.routes if getattr(route, "path", "") == "/internal/judge/dispatch")
        trace_route = next(route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/trace")
        replay_report_route = next(
            route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/replay/report"
        )
        replay_reports_route = next(
            route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/replay/reports"
        )

        dispatch_result = await dispatch_route.endpoint(request=request, x_ai_internal_key="m7-k1")
        self.assertTrue(dispatch_result["accepted"])
        self.assertEqual(dispatch_result["jobId"], 1)
        self.assertEqual(dispatch_result["winner"], "pro")
        self.assertEqual(len(callback_report_calls), 1)
        self.assertEqual(len(callback_failed_calls), 0)

        trace = await trace_route.endpoint(job_id=1, x_ai_internal_key="m7-k1")
        self.assertEqual(trace["status"], "completed")
        self.assertEqual(trace["callbackStatus"], "reported")

        replay_report = await replay_report_route.endpoint(job_id=1, x_ai_internal_key="m7-k1")
        self.assertEqual(replay_report["status"], "completed")
        self.assertEqual(replay_report["pipeline"]["finalWinner"], "pro")
        self.assertIn("judgeAudit", replay_report)

        replay_reports = await replay_reports_route.endpoint(
            x_ai_internal_key="m7-k1",
            status="completed",
            winner="pro",
            callback_status="reported",
            trace_id=None,
            created_after=None,
            created_before=None,
            has_audit_alert=None,
            limit=20,
            include_report=False,
        )
        self.assertEqual(replay_reports["count"], 1)
        self.assertEqual(replay_reports["items"][0]["jobId"], 1)
        self.assertEqual(replay_reports["items"][0]["winner"], "pro")

    async def test_m7_fault_injection_should_mark_failed_with_retry_and_error_code(self) -> None:
        settings = _build_settings(
            ai_internal_key="m7-k2",
            provider="openai",
            openai_api_key="test-key",
            openai_fallback_to_mock=False,
            fault_injection_nodes=("provider_timeout",),
        )
        callback_report_calls: list[tuple[int, dict]] = []
        callback_failed_calls: list[tuple[int, str]] = []

        async def fake_callback_report(*, cfg: object, job_id: int, payload: dict) -> None:
            callback_report_calls.append((job_id, payload))

        async def fake_callback_failed(*, cfg: object, job_id: int, error_message: str) -> None:
            callback_failed_calls.append((job_id, error_message))

        runtime = create_runtime(
            settings=settings,
            callback_report_impl=fake_callback_report,
            callback_failed_impl=fake_callback_failed,
        )
        app = create_app(runtime)
        request = _build_request()

        dispatch_route = next(route for route in app.routes if getattr(route, "path", "") == "/internal/judge/dispatch")
        trace_route = next(route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/trace")

        result = await dispatch_route.endpoint(request=request, x_ai_internal_key="m7-k2")
        self.assertEqual(result["status"], "marked_failed")
        self.assertEqual(result["errorCode"], "judge_timeout")
        self.assertEqual(result["attemptCount"], 2)
        self.assertEqual(result["retryCount"], 1)
        self.assertEqual(len(callback_report_calls), 0)
        self.assertEqual(len(callback_failed_calls), 1)
        self.assertIn("judge_timeout", callback_failed_calls[0][1])

        trace = await trace_route.endpoint(job_id=1, x_ai_internal_key="m7-k2")
        self.assertEqual(trace["status"], "failed")
        self.assertEqual(trace["callbackStatus"], "marked_failed")

    async def test_m7_compliance_alert_should_support_ack_resolve_and_outbox_delivery(self) -> None:
        settings = _build_settings(ai_internal_key="m7-k3")

        async def fake_runtime_builder(**_kwargs: object) -> _FakeReport:
            report = _FakeReport()
            report.payload["agentPipeline"] = {
                "compliance": {
                    "status": "warn",
                    "violations": ["display_missing_rationale"],
                }
            }
            return report

        async def fake_callback_report(*, cfg: object, job_id: int, payload: dict) -> None:
            return None

        async def fake_callback_failed(*, cfg: object, job_id: int, error_message: str) -> None:
            return None

        runtime = create_runtime(
            settings=settings,
            build_report_by_runtime_fn=fake_runtime_builder,
            callback_report_impl=fake_callback_report,
            callback_failed_impl=fake_callback_failed,
        )
        app = create_app(runtime)
        request = _build_request()

        dispatch_route = next(route for route in app.routes if getattr(route, "path", "") == "/internal/judge/dispatch")
        alerts_route = next(
            route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/alerts"
        )
        ack_route = next(
            route for route in app.routes if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/alerts/{alert_id}/ack"
        )
        resolve_route = next(
            route
            for route in app.routes
            if getattr(route, "path", "") == "/internal/judge/jobs/{job_id}/alerts/{alert_id}/resolve"
        )
        outbox_route = next(
            route for route in app.routes if getattr(route, "path", "") == "/internal/judge/alerts/outbox"
        )
        delivery_route = next(
            route
            for route in app.routes
            if getattr(route, "path", "") == "/internal/judge/alerts/outbox/{event_id}/delivery"
        )

        result = await dispatch_route.endpoint(request=request, x_ai_internal_key="m7-k3")
        self.assertEqual(result["status"], "marked_failed")
        self.assertEqual(result["errorCode"], "consistency_conflict")
        self.assertTrue(result["complianceBlocked"])
        self.assertEqual(len(result["auditAlertIds"]), 1)
        alert_id = result["auditAlertIds"][0]

        raised_rows = await alerts_route.endpoint(job_id=1, x_ai_internal_key="m7-k3", status="raised", limit=20)
        self.assertEqual(raised_rows["count"], 1)
        self.assertEqual(raised_rows["items"][0]["status"], "raised")

        acked = await ack_route.endpoint(
            job_id=1,
            alert_id=alert_id,
            x_ai_internal_key="m7-k3",
            actor="ops_reviewer_1",
            reason="reviewed",
        )
        self.assertEqual(acked["status"], "acked")

        resolved = await resolve_route.endpoint(
            job_id=1,
            alert_id=alert_id,
            x_ai_internal_key="m7-k3",
            actor="ops_reviewer_1",
            reason="fixed",
        )
        self.assertEqual(resolved["status"], "resolved")

        pending_outbox = await outbox_route.endpoint(
            x_ai_internal_key="m7-k3",
            delivery_status="pending",
            limit=20,
        )
        self.assertGreaterEqual(pending_outbox["count"], 3)
        event_id = pending_outbox["items"][0]["eventId"]

        marked = await delivery_route.endpoint(
            event_id=event_id,
            x_ai_internal_key="m7-k3",
            delivery_status="sent",
            error_message=None,
        )
        self.assertEqual(marked["item"]["deliveryStatus"], "sent")


if __name__ == "__main__":
    unittest.main()
