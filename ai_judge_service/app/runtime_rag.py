from __future__ import annotations

from typing import Callable

from .models import JudgeDispatchRequest, SubmitJudgeReportInput
from .rag_retriever import (
    RAG_BACKEND_MILVUS,
    RagMilvusConfig,
    RetrievedContext,
    retrieve_contexts,
    summarize_retrieved_contexts,
)
from .settings import Settings

RetrieveContextsFn = Callable[..., list[RetrievedContext]]


def build_milvus_config(settings: Settings) -> RagMilvusConfig | None:
    if (
        settings.rag_backend != RAG_BACKEND_MILVUS
        or not settings.rag_milvus_uri.strip()
        or not settings.rag_milvus_collection.strip()
    ):
        return None
    return RagMilvusConfig(
        uri=settings.rag_milvus_uri,
        token=settings.rag_milvus_token,
        db_name=settings.rag_milvus_db_name,
        collection=settings.rag_milvus_collection,
        vector_field=settings.rag_milvus_vector_field,
        content_field=settings.rag_milvus_content_field,
        title_field=settings.rag_milvus_title_field,
        source_url_field=settings.rag_milvus_source_url_field,
        chunk_id_field=settings.rag_milvus_chunk_id_field,
        tags_field=settings.rag_milvus_tags_field,
        metric_type=settings.rag_milvus_metric_type,
        search_limit=settings.rag_milvus_search_limit,
    )


def retrieve_runtime_contexts(
    *,
    request: JudgeDispatchRequest,
    settings: Settings,
    retrieve_contexts_fn: RetrieveContextsFn = retrieve_contexts,
) -> list[RetrievedContext]:
    return retrieve_contexts_fn(
        request,
        enabled=settings.rag_enabled,
        knowledge_file=settings.rag_knowledge_file,
        max_snippets=settings.rag_max_snippets,
        max_chars_per_snippet=settings.rag_max_chars_per_snippet,
        query_message_limit=settings.rag_query_message_limit,
        allowed_source_prefixes=settings.rag_source_whitelist,
        backend=settings.rag_backend,
        milvus_config=build_milvus_config(settings),
        openai_api_key=settings.openai_api_key,
        openai_base_url=settings.openai_base_url,
        openai_embedding_model=settings.rag_openai_embedding_model,
        openai_timeout_secs=settings.openai_timeout_secs,
    )


def apply_rag_payload_fields(
    report: SubmitJudgeReportInput,
    settings: Settings,
    retrieved_contexts: list[RetrievedContext],
    *,
    used_by_model: bool,
) -> None:
    report.payload["ragEnabled"] = settings.rag_enabled
    report.payload["ragBackend"] = settings.rag_backend
    report.payload["ragUsedByModel"] = used_by_model and bool(retrieved_contexts)
    report.payload["ragSnippetCount"] = len(retrieved_contexts)
    report.payload["ragSources"] = summarize_retrieved_contexts(retrieved_contexts)
    report.payload["ragSourceWhitelist"] = list(settings.rag_source_whitelist)
