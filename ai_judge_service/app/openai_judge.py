from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import httpx

from .openai_judge_helpers import (
    _build_aggregate_system_prompt,
    _build_aggregate_user_prompt,
    _build_display_system_prompt,
    _build_display_user_prompt,
    _build_final_system_prompt,
    _build_final_user_prompt,
    _build_stage_summary_fallback,
    _build_stage_system_prompt,
    _build_stage_user_prompt,
    _build_user_prompt,
    _extract_json_object,
    _merge_two_pass,
    _normalize_aggregate_eval,
    _normalize_display_eval,
    _normalize_eval,
    _normalize_stage_eval,
    _split_message_chunks,
)
from .rag_retriever import RetrievedContext, summarize_retrieved_contexts
from .scoring_core import DebateMessage

if TYPE_CHECKING:
    from .models import JudgeDispatchRequest, SubmitJudgeReportInput


@dataclass(frozen=True)
class OpenAiJudgeConfig:
    api_key: str
    model: str
    base_url: str
    timeout_secs: float
    temperature: float
    max_retries: int
    max_stage_agent_chunks: int = 12


async def _call_openai_json(
    *,
    cfg: OpenAiJudgeConfig,
    system_prompt: str,
    user_prompt: str,
) -> dict[str, Any]:
    body = {
        "model": cfg.model,
        "temperature": cfg.temperature,
        "response_format": {"type": "json_object"},
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
    }
    headers = {
        "Authorization": f"Bearer {cfg.api_key}",
        "Content-Type": "application/json",
    }
    last_err: Exception | None = None
    for _ in range(max(1, cfg.max_retries)):
        try:
            async with httpx.AsyncClient(timeout=cfg.timeout_secs) as client:
                resp = await client.post(
                    f"{cfg.base_url}/chat/completions",
                    headers=headers,
                    json=body,
                )
            if resp.status_code // 100 != 2:
                raise RuntimeError(f"openai status={resp.status_code}, body={resp.text[:500]}")
            data = resp.json()
            content = data["choices"][0]["message"]["content"]
            return _extract_json_object(content)
        except Exception as err:  # pragma: no cover
            last_err = err
    raise RuntimeError(f"openai call failed after retries: {last_err}")


async def _build_stage_summaries_with_openai(
    *,
    cfg: OpenAiJudgeConfig,
    request: "JudgeDispatchRequest",
    messages: list[DebateMessage],
    retrieved_contexts: list[RetrievedContext],
    style_mode: str,
) -> tuple[list[dict[str, Any]], int]:
    chunks = _split_message_chunks(
        messages,
        request.message_window_size,
        cfg.max_stage_agent_chunks,
    )
    if not chunks:
        return [], 0

    stage_summaries: list[dict[str, Any]] = []
    fallback_count = 0
    stage_count = len(chunks)
    for stage_no, chunk in chunks:
        try:
            raw = await _call_openai_json(
                cfg=cfg,
                system_prompt=_build_stage_system_prompt(style_mode, stage_no),
                user_prompt=_build_stage_user_prompt(
                    request,
                    chunk,
                    retrieved_contexts,
                    stage_no,
                    stage_count,
                ),
            )
            stage_summaries.append(_normalize_stage_eval(raw, chunk, stage_no))
        except Exception:
            fallback_count += 1
            stage_summaries.append(_build_stage_summary_fallback(chunk, stage_no))

    return stage_summaries, fallback_count


async def _build_aggregate_summary_with_openai(
    *,
    cfg: OpenAiJudgeConfig,
    request: "JudgeDispatchRequest",
    stage_summaries: list[dict[str, Any]],
    retrieved_contexts: list[RetrievedContext],
    style_mode: str,
) -> tuple[dict[str, Any], bool]:
    fallback = False
    try:
        raw = await _call_openai_json(
            cfg=cfg,
            system_prompt=_build_aggregate_system_prompt(style_mode),
            user_prompt=_build_aggregate_user_prompt(request, stage_summaries, retrieved_contexts),
        )
    except Exception:
        raw = {}
        fallback = True
    return _normalize_aggregate_eval(raw, stage_summaries), fallback


async def _call_openai_final_pass(
    *,
    cfg: OpenAiJudgeConfig,
    request: "JudgeDispatchRequest",
    stage_summaries: list[dict[str, Any]],
    aggregate_summary: dict[str, Any],
    retrieved_contexts: list[RetrievedContext],
    style_mode: str,
    pass_no: int,
) -> dict[str, Any]:
    return await _call_openai_json(
        cfg=cfg,
        system_prompt=_build_final_system_prompt(style_mode, pass_no),
        user_prompt=_build_final_user_prompt(
            request,
            stage_summaries,
            aggregate_summary,
            retrieved_contexts,
        ),
    )


async def build_report_with_openai(
    *,
    request: "JudgeDispatchRequest",
    effective_style_mode: str,
    style_mode_source: str,
    cfg: OpenAiJudgeConfig,
    retrieved_contexts: list[RetrievedContext] | None = None,
) -> "SubmitJudgeReportInput":
    from .models import SubmitJudgeReportInput

    retrieved_contexts = retrieved_contexts or []
    messages = [
        DebateMessage(
            message_id=msg.message_id,
            user_id=msg.user_id,
            side=msg.side,
            content=msg.content,
        )
        for msg in request.messages
    ]

    stage_summaries, stage_fallback_count = await _build_stage_summaries_with_openai(
        cfg=cfg,
        request=request,
        messages=messages,
        retrieved_contexts=retrieved_contexts,
        style_mode=effective_style_mode,
    )
    aggregate_summary, aggregate_fallback = await _build_aggregate_summary_with_openai(
        cfg=cfg,
        request=request,
        stage_summaries=stage_summaries,
        retrieved_contexts=retrieved_contexts,
        style_mode=effective_style_mode,
    )

    first_raw = await _call_openai_final_pass(
        cfg=cfg,
        request=request,
        stage_summaries=stage_summaries,
        aggregate_summary=aggregate_summary,
        retrieved_contexts=retrieved_contexts,
        style_mode=effective_style_mode,
        pass_no=1,
    )
    second_raw = await _call_openai_final_pass(
        cfg=cfg,
        request=request,
        stage_summaries=stage_summaries,
        aggregate_summary=aggregate_summary,
        retrieved_contexts=retrieved_contexts,
        style_mode=effective_style_mode,
        pass_no=2,
    )

    first = _normalize_eval(first_raw)
    second = _normalize_eval(second_raw)
    merged = _merge_two_pass(first, second)

    display_fallback = False
    try:
        display_raw = await _call_openai_json(
            cfg=cfg,
            system_prompt=_build_display_system_prompt(effective_style_mode),
            user_prompt=_build_display_user_prompt(merged, aggregate_summary),
        )
    except Exception:
        display_raw = {}
        display_fallback = True
    display = _normalize_display_eval(display_raw, merged)

    payload = {
        "provider": "openai",
        "model": cfg.model,
        "winnerFirst": merged["winner_first"],
        "winnerSecond": merged["winner_second"],
        "rubricVersion": request.rubric_version,
        "requestedStyleMode": request.job.style_mode,
        "effectiveStyleMode": effective_style_mode,
        "styleModeSource": style_mode_source,
        "ragEnabled": True,
        "ragUsedByModel": bool(retrieved_contexts),
        "ragSnippetCount": len(retrieved_contexts),
        "ragSources": summarize_retrieved_contexts(retrieved_contexts),
        "agentPipelineVersion": "multi-agent-v1",
        "agentPipeline": {
            "stageAgent": "openai",
            "aggregateAgent": "openai" if not aggregate_fallback else "fallback",
            "finalJudgeAgent": "openai",
            "displayAgent": "openai" if not display_fallback else "fallback",
            "stageCount": len(stage_summaries),
            "stageFallbackCount": stage_fallback_count,
            "maxStageAgentChunks": cfg.max_stage_agent_chunks,
        },
        "aggregateSummary": {
            "winnerHint": aggregate_summary["winner_hint"],
            "proScoreHint": aggregate_summary["pro_score_hint"],
            "conScoreHint": aggregate_summary["con_score_hint"],
        },
    }

    return SubmitJudgeReportInput(
        winner=merged["winner"],
        pro_score=merged["pro_score"],
        con_score=merged["con_score"],
        logic_pro=merged["logic_pro"],
        logic_con=merged["logic_con"],
        evidence_pro=merged["evidence_pro"],
        evidence_con=merged["evidence_con"],
        rebuttal_pro=merged["rebuttal_pro"],
        rebuttal_con=merged["rebuttal_con"],
        clarity_pro=merged["clarity_pro"],
        clarity_con=merged["clarity_con"],
        pro_summary=display["pro_summary"],
        con_summary=display["con_summary"],
        rationale=display["rationale"],
        style_mode=effective_style_mode,
        needs_draw_vote=merged["needs_draw_vote"],
        rejudge_triggered=request.job.rejudge_triggered or merged["rejudge_triggered"],
        payload=payload,
        winner_first=merged["winner_first"],
        winner_second=merged["winner_second"],
        stage_summaries=stage_summaries,
    )
