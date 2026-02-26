import asyncio
import os
from dataclasses import dataclass

from fastapi import FastAPI, Header, HTTPException

from .callback_client import CallbackClientConfig, callback_failed, callback_report
from .dispatch_controller import DispatchRuntimeConfig, process_dispatch_request
from .models import JudgeDispatchRequest
from .openai_judge import OpenAiJudgeConfig, build_report_with_openai
from .rag_retriever import (
    RAG_BACKEND_MILVUS,
    RagMilvusConfig,
    parse_rag_backend,
    parse_source_whitelist,
    retrieve_contexts,
    summarize_retrieved_contexts,
)
from .runtime_policy import PROVIDER_OPENAI, normalize_provider, parse_env_bool, should_use_openai
from .scoring import build_report

DEFAULT_RAG_SOURCE_WHITELIST = "https://teamfighttactics.leagueoflegends.com/en-us/news/"


@dataclass(frozen=True)
class Settings:
    ai_internal_key: str
    chat_server_base_url: str
    report_path_template: str
    failed_path_template: str
    callback_timeout_secs: float
    process_delay_ms: int
    judge_style_mode: str
    provider: str
    openai_api_key: str
    openai_model: str
    openai_base_url: str
    openai_timeout_secs: float
    openai_temperature: float
    openai_max_retries: int
    openai_fallback_to_mock: bool
    rag_enabled: bool
    rag_knowledge_file: str
    rag_max_snippets: int
    rag_max_chars_per_snippet: int
    rag_query_message_limit: int
    rag_source_whitelist: tuple[str, ...]
    rag_backend: str
    rag_openai_embedding_model: str
    rag_milvus_uri: str
    rag_milvus_token: str
    rag_milvus_db_name: str
    rag_milvus_collection: str
    rag_milvus_vector_field: str
    rag_milvus_content_field: str
    rag_milvus_title_field: str
    rag_milvus_source_url_field: str
    rag_milvus_chunk_id_field: str
    rag_milvus_tags_field: str
    rag_milvus_metric_type: str
    rag_milvus_search_limit: int
    stage_agent_max_chunks: int


def _load_settings() -> Settings:
    provider = normalize_provider(os.getenv("AI_JUDGE_PROVIDER", "mock"))
    return Settings(
        ai_internal_key=os.getenv("AI_JUDGE_INTERNAL_KEY", "dev-ai-internal-key"),
        chat_server_base_url=os.getenv("CHAT_SERVER_BASE_URL", "http://127.0.0.1:6688"),
        report_path_template=os.getenv(
            "CHAT_SERVER_REPORT_PATH_TEMPLATE",
            "/api/internal/ai/judge/jobs/{job_id}/report",
        ),
        failed_path_template=os.getenv(
            "CHAT_SERVER_FAILED_PATH_TEMPLATE",
            "/api/internal/ai/judge/jobs/{job_id}/failed",
        ),
        callback_timeout_secs=float(os.getenv("CALLBACK_TIMEOUT_SECONDS", "8")),
        process_delay_ms=int(os.getenv("JUDGE_PROCESS_DELAY_MS", "0")),
        judge_style_mode=os.getenv("JUDGE_STYLE_MODE", "rational"),
        provider=provider,
        openai_api_key=os.getenv("OPENAI_API_KEY", ""),
        openai_model=os.getenv("AI_JUDGE_OPENAI_MODEL", "gpt-4.1-mini"),
        openai_base_url=os.getenv("AI_JUDGE_OPENAI_BASE_URL", "https://api.openai.com/v1").rstrip("/"),
        openai_timeout_secs=float(os.getenv("AI_JUDGE_OPENAI_TIMEOUT_SECONDS", "25")),
        openai_temperature=float(os.getenv("AI_JUDGE_OPENAI_TEMPERATURE", "0.1")),
        openai_max_retries=int(os.getenv("AI_JUDGE_OPENAI_MAX_RETRIES", "2")),
        openai_fallback_to_mock=parse_env_bool(
            os.getenv("AI_JUDGE_OPENAI_FALLBACK_TO_MOCK"),
            default=True,
        ),
        rag_enabled=parse_env_bool(os.getenv("AI_JUDGE_RAG_ENABLED"), default=True),
        rag_knowledge_file=os.getenv("AI_JUDGE_RAG_KNOWLEDGE_FILE", ""),
        rag_max_snippets=int(os.getenv("AI_JUDGE_RAG_MAX_SNIPPETS", "4")),
        rag_max_chars_per_snippet=int(os.getenv("AI_JUDGE_RAG_MAX_CHARS_PER_SNIPPET", "280")),
        rag_query_message_limit=int(os.getenv("AI_JUDGE_RAG_QUERY_MESSAGE_LIMIT", "80")),
        rag_source_whitelist=parse_source_whitelist(
            os.getenv(
                "AI_JUDGE_RAG_SOURCE_WHITELIST",
                DEFAULT_RAG_SOURCE_WHITELIST,
            )
        ),
        rag_backend=parse_rag_backend(os.getenv("AI_JUDGE_RAG_BACKEND", "file")),
        rag_openai_embedding_model=os.getenv(
            "AI_JUDGE_RAG_OPENAI_EMBEDDING_MODEL",
            "text-embedding-3-small",
        ),
        rag_milvus_uri=os.getenv("AI_JUDGE_RAG_MILVUS_URI", ""),
        rag_milvus_token=os.getenv("AI_JUDGE_RAG_MILVUS_TOKEN", ""),
        rag_milvus_db_name=os.getenv("AI_JUDGE_RAG_MILVUS_DB_NAME", ""),
        rag_milvus_collection=os.getenv("AI_JUDGE_RAG_MILVUS_COLLECTION", ""),
        rag_milvus_vector_field=os.getenv("AI_JUDGE_RAG_MILVUS_VECTOR_FIELD", "embedding"),
        rag_milvus_content_field=os.getenv("AI_JUDGE_RAG_MILVUS_CONTENT_FIELD", "content"),
        rag_milvus_title_field=os.getenv("AI_JUDGE_RAG_MILVUS_TITLE_FIELD", "title"),
        rag_milvus_source_url_field=os.getenv(
            "AI_JUDGE_RAG_MILVUS_SOURCE_URL_FIELD",
            "source_url",
        ),
        rag_milvus_chunk_id_field=os.getenv("AI_JUDGE_RAG_MILVUS_CHUNK_ID_FIELD", "chunk_id"),
        rag_milvus_tags_field=os.getenv("AI_JUDGE_RAG_MILVUS_TAGS_FIELD", "tags"),
        rag_milvus_metric_type=os.getenv("AI_JUDGE_RAG_MILVUS_METRIC_TYPE", "COSINE"),
        rag_milvus_search_limit=int(os.getenv("AI_JUDGE_RAG_MILVUS_SEARCH_LIMIT", "20")),
        stage_agent_max_chunks=int(os.getenv("AI_JUDGE_STAGE_AGENT_MAX_CHUNKS", "12")),
    )


SETTINGS = _load_settings()
app = FastAPI(title="AI Judge Service", version="0.2.0")
CALLBACK_CFG = CallbackClientConfig(
    ai_internal_key=SETTINGS.ai_internal_key,
    chat_server_base_url=SETTINGS.chat_server_base_url,
    report_path_template=SETTINGS.report_path_template,
    failed_path_template=SETTINGS.failed_path_template,
    callback_timeout_secs=SETTINGS.callback_timeout_secs,
)
DISPATCH_RUNTIME_CFG = DispatchRuntimeConfig(
    process_delay_ms=SETTINGS.process_delay_ms,
    judge_style_mode=SETTINGS.judge_style_mode,
)


def _require_internal_key(header_value: str | None) -> None:
    if not header_value:
        raise HTTPException(status_code=401, detail="missing x-ai-internal-key")
    if header_value.strip() != SETTINGS.ai_internal_key:
        raise HTTPException(status_code=401, detail="invalid x-ai-internal-key")


async def _build_report_by_runtime(
    request: JudgeDispatchRequest,
    effective_style_mode: str,
    style_mode_source: str,
):
    milvus_config: RagMilvusConfig | None = None
    if (
        SETTINGS.rag_backend == RAG_BACKEND_MILVUS
        and SETTINGS.rag_milvus_uri.strip()
        and SETTINGS.rag_milvus_collection.strip()
    ):
        milvus_config = RagMilvusConfig(
            uri=SETTINGS.rag_milvus_uri,
            token=SETTINGS.rag_milvus_token,
            db_name=SETTINGS.rag_milvus_db_name,
            collection=SETTINGS.rag_milvus_collection,
            vector_field=SETTINGS.rag_milvus_vector_field,
            content_field=SETTINGS.rag_milvus_content_field,
            title_field=SETTINGS.rag_milvus_title_field,
            source_url_field=SETTINGS.rag_milvus_source_url_field,
            chunk_id_field=SETTINGS.rag_milvus_chunk_id_field,
            tags_field=SETTINGS.rag_milvus_tags_field,
            metric_type=SETTINGS.rag_milvus_metric_type,
            search_limit=SETTINGS.rag_milvus_search_limit,
        )

    retrieved_contexts = retrieve_contexts(
        request,
        enabled=SETTINGS.rag_enabled,
        knowledge_file=SETTINGS.rag_knowledge_file,
        max_snippets=SETTINGS.rag_max_snippets,
        max_chars_per_snippet=SETTINGS.rag_max_chars_per_snippet,
        query_message_limit=SETTINGS.rag_query_message_limit,
        allowed_source_prefixes=SETTINGS.rag_source_whitelist,
        backend=SETTINGS.rag_backend,
        milvus_config=milvus_config,
        openai_api_key=SETTINGS.openai_api_key,
        openai_base_url=SETTINGS.openai_base_url,
        openai_embedding_model=SETTINGS.rag_openai_embedding_model,
        openai_timeout_secs=SETTINGS.openai_timeout_secs,
    )

    def apply_rag_payload_fields(report, *, used_by_model: bool) -> None:
        report.payload["ragEnabled"] = SETTINGS.rag_enabled
        report.payload["ragBackend"] = SETTINGS.rag_backend
        report.payload["ragUsedByModel"] = used_by_model and bool(retrieved_contexts)
        report.payload["ragSnippetCount"] = len(retrieved_contexts)
        report.payload["ragSources"] = summarize_retrieved_contexts(retrieved_contexts)
        report.payload["ragSourceWhitelist"] = list(SETTINGS.rag_source_whitelist)

    if should_use_openai(SETTINGS.provider, SETTINGS.openai_api_key):
        cfg = OpenAiJudgeConfig(
            api_key=SETTINGS.openai_api_key,
            model=SETTINGS.openai_model,
            base_url=SETTINGS.openai_base_url,
            timeout_secs=SETTINGS.openai_timeout_secs,
            temperature=SETTINGS.openai_temperature,
            max_retries=SETTINGS.openai_max_retries,
            max_stage_agent_chunks=SETTINGS.stage_agent_max_chunks,
        )
        try:
            report = await build_report_with_openai(
                request=request,
                effective_style_mode=effective_style_mode,
                style_mode_source=style_mode_source,
                cfg=cfg,
                retrieved_contexts=retrieved_contexts,
            )
        except Exception as err:
            if not SETTINGS.openai_fallback_to_mock:
                raise RuntimeError(f"openai runtime failed: {err}") from err
            report = build_report(request, system_style_mode=SETTINGS.judge_style_mode)
            report.payload["provider"] = "ai-judge-service-mock-fallback"
            report.payload["fallbackFrom"] = "openai"
            report.payload["fallbackReason"] = str(err)[:500]
            apply_rag_payload_fields(report, used_by_model=False)
            return report
        apply_rag_payload_fields(report, used_by_model=True)
        return report

    report = build_report(request, system_style_mode=SETTINGS.judge_style_mode)
    if SETTINGS.provider == PROVIDER_OPENAI and not SETTINGS.openai_api_key.strip():
        report.payload["provider"] = "ai-judge-service-mock-missing-openai-key"
        report.payload["fallbackFrom"] = "openai"
        report.payload["fallbackReason"] = "missing OPENAI_API_KEY"
    apply_rag_payload_fields(report, used_by_model=False)
    return report


@app.get("/healthz")
async def healthz() -> dict[str, bool]:
    return {"ok": True}


@app.post("/internal/judge/dispatch")
async def dispatch_judge_job(
    request: JudgeDispatchRequest,
    x_ai_internal_key: str | None = Header(default=None),
) -> dict:
    _require_internal_key(x_ai_internal_key)
    return await process_dispatch_request(
        request=request,
        runtime_cfg=DISPATCH_RUNTIME_CFG,
        build_report_by_runtime=_build_report_by_runtime,
        callback_report=lambda job_id, payload: callback_report(
            cfg=CALLBACK_CFG,
            job_id=job_id,
            payload=payload,
        ),
        callback_failed=lambda job_id, error_message: callback_failed(
            cfg=CALLBACK_CFG,
            job_id=job_id,
            error_message=error_message,
        ),
        sleep_fn=asyncio.sleep,
    )
