from __future__ import annotations

from typing import Awaitable, Callable

from .models import JudgeDispatchRequest, SubmitJudgeReportInput
from .openai_judge import build_report_with_openai
from .rag_retriever import RetrievedContext, retrieve_contexts
from .runtime_provider import build_report_with_provider
from .runtime_rag import (
    apply_rag_payload_fields,
    retrieve_runtime_contexts,
)
from .scoring import build_report
from .settings import Settings

RetrieveContextsFn = Callable[..., list[RetrievedContext]]
BuildOpenAiReportFn = Callable[..., Awaitable[SubmitJudgeReportInput]]
BuildMockReportFn = Callable[..., SubmitJudgeReportInput]


async def build_report_by_runtime(
    *,
    request: JudgeDispatchRequest,
    effective_style_mode: str,
    style_mode_source: str,
    settings: Settings,
    retrieve_contexts_fn: RetrieveContextsFn = retrieve_contexts,
    build_report_with_openai_fn: BuildOpenAiReportFn = build_report_with_openai,
    build_mock_report_fn: BuildMockReportFn = build_report,
) -> SubmitJudgeReportInput:
    retrieved_contexts = retrieve_runtime_contexts(
        request=request,
        settings=settings,
        retrieve_contexts_fn=retrieve_contexts_fn,
    )
    report, used_by_model = await build_report_with_provider(
        request=request,
        effective_style_mode=effective_style_mode,
        style_mode_source=style_mode_source,
        settings=settings,
        retrieved_contexts=retrieved_contexts,
        build_report_with_openai_fn=build_report_with_openai_fn,
        build_mock_report_fn=build_mock_report_fn,
    )
    apply_rag_payload_fields(
        report,
        settings,
        retrieved_contexts,
        used_by_model=used_by_model,
    )
    return report
