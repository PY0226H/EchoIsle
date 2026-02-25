from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any

import httpx

if TYPE_CHECKING:
    from .models import JudgeDispatchRequest

TOKEN_RE = re.compile(r"[A-Za-z0-9]+|[\u4e00-\u9fff]+")
RAG_BACKEND_FILE = "file"
RAG_BACKEND_MILVUS = "milvus"


@dataclass(frozen=True)
class KnowledgeChunk:
    chunk_id: str
    title: str
    source_url: str
    content: str
    tags: tuple[str, ...]


@dataclass(frozen=True)
class RetrievedContext:
    chunk_id: str
    title: str
    source_url: str
    content: str
    score: float

    def to_payload_source(self) -> dict[str, Any]:
        return {
            "chunkId": self.chunk_id,
            "title": self.title,
            "sourceUrl": self.source_url,
            "score": round(self.score, 4),
        }


@dataclass(frozen=True)
class RagMilvusConfig:
    uri: str
    collection: str
    token: str = ""
    db_name: str = ""
    vector_field: str = "embedding"
    content_field: str = "content"
    title_field: str = "title"
    source_url_field: str = "source_url"
    chunk_id_field: str = "chunk_id"
    tags_field: str = "tags"
    metric_type: str = "COSINE"
    search_limit: int = 20


def _safe_text(value: Any, *, max_len: int = 4000) -> str:
    text = str(value or "").strip()
    if not text:
        return ""
    return text[:max_len]


def _safe_float(value: Any, *, default: float = 0.0) -> float:
    try:
        return float(value)
    except Exception:
        return default


def _tokenize(text: str) -> set[str]:
    out: set[str] = set()
    for token in TOKEN_RE.findall((text or "").lower()):
        token = token.strip()
        if len(token) < 2:
            continue
        if re.fullmatch(r"[\u4e00-\u9fff]+", token):
            out.add(token)
            # 兼容中文短语匹配，拆成 2-gram 提升召回率
            for idx in range(0, len(token) - 1):
                out.add(token[idx : idx + 2])
            continue
        out.add(token)
    return out


def _load_knowledge_file(path: str) -> list[KnowledgeChunk]:
    path = path.strip()
    if not path:
        return []
    file = Path(path)
    if not file.exists() or not file.is_file():
        return []

    try:
        payload = json.loads(file.read_text(encoding="utf-8"))
    except Exception:
        return []

    if not isinstance(payload, list):
        return []

    rows: list[KnowledgeChunk] = []
    for idx, row in enumerate(payload):
        if not isinstance(row, dict):
            continue
        chunk_id = _safe_text(
            row.get("chunk_id") or row.get("chunkId") or f"chunk-{idx + 1}",
            max_len=128,
        )
        title = _safe_text(row.get("title"), max_len=300) or chunk_id
        source_url = _safe_text(
            row.get("source_url") or row.get("sourceUrl"),
            max_len=1000,
        )
        content = _safe_text(row.get("content"), max_len=8000)
        if not content:
            continue
        raw_tags = row.get("tags")
        tags: tuple[str, ...]
        if isinstance(raw_tags, list):
            tags = tuple(
                v.strip().lower()
                for v in raw_tags
                if isinstance(v, str) and v.strip()
            )
        else:
            tags = tuple()
        rows.append(
            KnowledgeChunk(
                chunk_id=chunk_id,
                title=title,
                source_url=source_url,
                content=content,
                tags=tags,
            )
        )
    return rows


def _build_query_text(request: "JudgeDispatchRequest", query_message_limit: int) -> str:
    topic = request.topic
    topic_text = " ".join(
        [
            topic.title,
            topic.category,
            topic.description,
            topic.stance_pro,
            topic.stance_con,
            topic.context_seed or "",
        ]
    )
    msg_count = max(0, query_message_limit)
    selected_messages = request.messages if msg_count == 0 else request.messages[-msg_count:]
    message_text = " ".join(msg.content for msg in selected_messages)
    return f"{topic_text}\n{message_text}"


def _build_query_tokens(request: "JudgeDispatchRequest", query_message_limit: int) -> set[str]:
    return _tokenize(_build_query_text(request, query_message_limit))


def _score_chunk(chunk: KnowledgeChunk, query_tokens: set[str]) -> float:
    if not query_tokens:
        return 0.0
    chunk_tokens = _tokenize(
        " ".join([chunk.title, chunk.content, " ".join(chunk.tags)])
    )
    if not chunk_tokens:
        return 0.0
    overlap = len(query_tokens.intersection(chunk_tokens))
    if overlap == 0:
        return 0.0

    coverage = overlap / max(1, len(query_tokens))
    density = overlap / max(1, len(chunk_tokens))
    return coverage * 0.75 + density * 0.25


def parse_source_whitelist(raw: str | None) -> tuple[str, ...]:
    text = (raw or "").strip()
    if not text:
        return tuple()

    parts = re.split(r"[,\n;]+", text)
    normalized: list[str] = []
    seen: set[str] = set()
    for part in parts:
        item = part.strip().rstrip("/")
        if not item:
            continue
        lowered = item.lower()
        if lowered in seen:
            continue
        seen.add(lowered)
        normalized.append(item)
    return tuple(normalized)


def parse_rag_backend(raw: str | None) -> str:
    value = (raw or "").strip().lower()
    if value == RAG_BACKEND_MILVUS:
        return RAG_BACKEND_MILVUS
    return RAG_BACKEND_FILE


def _source_allowed(source_url: str, whitelist_prefixes: tuple[str, ...]) -> bool:
    if not whitelist_prefixes:
        return True
    source = (source_url or "").strip().lower()
    if not source:
        return False
    for prefix in whitelist_prefixes:
        prefix_norm = prefix.strip().lower().rstrip("/")
        if not prefix_norm:
            continue
        if source == prefix_norm or source.startswith(prefix_norm + "/"):
            return True
    return False


def _score_rank(metric_type: str, distance: float, rank: int) -> float:
    if rank < 0:
        rank = 0
    metric = metric_type.strip().upper()
    if metric in {"L2", "EUCLIDEAN"}:
        return 1.0 / (1.0 + max(0.0, distance))
    return max(0.0, distance) + (1.0 / (rank + 1000.0))


def _embed_query_with_openai(
    *,
    query_text: str,
    openai_api_key: str,
    openai_base_url: str,
    openai_embedding_model: str,
    openai_timeout_secs: float,
) -> list[float]:
    if not query_text.strip() or not openai_api_key.strip():
        return []

    url = openai_base_url.rstrip("/") + "/embeddings"
    headers = {
        "Authorization": f"Bearer {openai_api_key}",
        "Content-Type": "application/json",
    }
    body = {
        "model": openai_embedding_model,
        "input": query_text[:8000],
    }
    try:
        with httpx.Client(timeout=max(1.0, openai_timeout_secs)) as client:
            resp = client.post(url, headers=headers, json=body)
        if resp.status_code // 100 != 2:
            return []
        payload = resp.json()
        items = payload.get("data")
        if not isinstance(items, list) or not items:
            return []
        embedding = items[0].get("embedding")
        if not isinstance(embedding, list):
            return []
        out = [_safe_float(value) for value in embedding]
        return out if out else []
    except Exception:
        return []


def _fetch_milvus_candidates(
    *,
    query_embedding: list[float],
    cfg: RagMilvusConfig,
) -> list[dict[str, Any]]:
    if not query_embedding:
        return []
    try:
        from pymilvus import MilvusClient  # type: ignore
    except Exception:
        return []

    try:
        client = MilvusClient(
            uri=cfg.uri,
            token=cfg.token or None,
            db_name=cfg.db_name or None,
        )
        raw = client.search(
            collection_name=cfg.collection,
            data=[query_embedding],
            anns_field=cfg.vector_field,
            limit=max(1, cfg.search_limit),
            search_params={"metric_type": cfg.metric_type},
            output_fields=[
                cfg.chunk_id_field,
                cfg.title_field,
                cfg.source_url_field,
                cfg.content_field,
                cfg.tags_field,
            ],
        )
    except Exception:
        return []

    if not isinstance(raw, list) or not raw:
        return []
    first = raw[0]
    if not isinstance(first, list):
        return []
    return [row for row in first if isinstance(row, dict)]


def _context_from_milvus_row(
    row: dict[str, Any],
    *,
    cfg: RagMilvusConfig,
    rank: int,
    max_chars_per_snippet: int,
) -> RetrievedContext | None:
    entity = row.get("entity")
    fields = entity if isinstance(entity, dict) else row
    chunk_id = _safe_text(
        fields.get(cfg.chunk_id_field) or row.get("id") or f"milvus-{rank + 1}",
        max_len=128,
    )
    title = _safe_text(fields.get(cfg.title_field), max_len=300) or chunk_id
    source_url = _safe_text(fields.get(cfg.source_url_field), max_len=1000)
    content = _safe_text(fields.get(cfg.content_field), max_len=max_chars_per_snippet)
    if not content:
        return None
    distance = _safe_float(row.get("distance"), default=0.0)
    score = _score_rank(cfg.metric_type, distance, rank)
    return RetrievedContext(
        chunk_id=chunk_id,
        title=title,
        source_url=source_url,
        content=content,
        score=score,
    )


def _retrieve_contexts_from_file(
    request: "JudgeDispatchRequest",
    *,
    knowledge_file: str,
    max_snippets: int,
    max_chars_per_snippet: int,
    query_message_limit: int,
    allowed_source_prefixes: tuple[str, ...] = (),
) -> list[RetrievedContext]:
    out: list[RetrievedContext] = []
    seen_chunk_ids: set[str] = set()

    context_seed = _safe_text(request.topic.context_seed, max_len=max_chars_per_snippet)
    if context_seed:
        out.append(
            RetrievedContext(
                chunk_id="topic-context-seed",
                title="topic_context_seed",
                source_url="request.topic.context_seed",
                content=context_seed,
                score=1.0,
            )
        )
        seen_chunk_ids.add("topic-context-seed")
        if len(out) >= max_snippets:
            return out

    chunks = _load_knowledge_file(knowledge_file)
    query_tokens = _build_query_tokens(request, query_message_limit)
    ranked: list[RetrievedContext] = []
    for chunk in chunks:
        if not _source_allowed(chunk.source_url, allowed_source_prefixes):
            continue
        score = _score_chunk(chunk, query_tokens)
        if score <= 0:
            continue
        ranked.append(
            RetrievedContext(
                chunk_id=chunk.chunk_id,
                title=chunk.title,
                source_url=chunk.source_url,
                content=_safe_text(chunk.content, max_len=max_chars_per_snippet),
                score=score,
            )
        )

    ranked.sort(key=lambda item: (-item.score, item.chunk_id))
    for item in ranked:
        if item.chunk_id in seen_chunk_ids:
            continue
        out.append(item)
        seen_chunk_ids.add(item.chunk_id)
        if len(out) >= max_snippets:
            break
    return out


def _retrieve_contexts_from_milvus(
    request: "JudgeDispatchRequest",
    *,
    max_snippets: int,
    max_chars_per_snippet: int,
    query_message_limit: int,
    allowed_source_prefixes: tuple[str, ...],
    milvus_config: RagMilvusConfig,
    openai_api_key: str,
    openai_base_url: str,
    openai_embedding_model: str,
    openai_timeout_secs: float,
) -> list[RetrievedContext]:
    out: list[RetrievedContext] = []
    seen_chunk_ids: set[str] = set()

    context_seed = _safe_text(request.topic.context_seed, max_len=max_chars_per_snippet)
    if context_seed:
        out.append(
            RetrievedContext(
                chunk_id="topic-context-seed",
                title="topic_context_seed",
                source_url="request.topic.context_seed",
                content=context_seed,
                score=1.0,
            )
        )
        seen_chunk_ids.add("topic-context-seed")
        if len(out) >= max_snippets:
            return out

    query_text = _build_query_text(request, query_message_limit)
    embedding = _embed_query_with_openai(
        query_text=query_text,
        openai_api_key=openai_api_key,
        openai_base_url=openai_base_url,
        openai_embedding_model=openai_embedding_model,
        openai_timeout_secs=openai_timeout_secs,
    )
    rows = _fetch_milvus_candidates(query_embedding=embedding, cfg=milvus_config)
    for rank, row in enumerate(rows):
        item = _context_from_milvus_row(
            row,
            cfg=milvus_config,
            rank=rank,
            max_chars_per_snippet=max_chars_per_snippet,
        )
        if item is None:
            continue
        if not _source_allowed(item.source_url, allowed_source_prefixes):
            continue
        if item.chunk_id in seen_chunk_ids:
            continue
        out.append(item)
        seen_chunk_ids.add(item.chunk_id)
        if len(out) >= max_snippets:
            break

    return out


def retrieve_contexts(
    request: "JudgeDispatchRequest",
    *,
    enabled: bool,
    knowledge_file: str,
    max_snippets: int,
    max_chars_per_snippet: int,
    query_message_limit: int,
    allowed_source_prefixes: tuple[str, ...] = (),
    backend: str = RAG_BACKEND_FILE,
    milvus_config: RagMilvusConfig | None = None,
    openai_api_key: str = "",
    openai_base_url: str = "https://api.openai.com/v1",
    openai_embedding_model: str = "text-embedding-3-small",
    openai_timeout_secs: float = 8.0,
) -> list[RetrievedContext]:
    if not enabled:
        return []

    limit = max(0, max_snippets)
    if limit == 0:
        return []

    selected_backend = parse_rag_backend(backend)
    if selected_backend == RAG_BACKEND_MILVUS and milvus_config is not None:
        return _retrieve_contexts_from_milvus(
            request,
            max_snippets=limit,
            max_chars_per_snippet=max_chars_per_snippet,
            query_message_limit=query_message_limit,
            allowed_source_prefixes=allowed_source_prefixes,
            milvus_config=milvus_config,
            openai_api_key=openai_api_key,
            openai_base_url=openai_base_url,
            openai_embedding_model=openai_embedding_model,
            openai_timeout_secs=openai_timeout_secs,
        )
    return _retrieve_contexts_from_file(
        request,
        knowledge_file=knowledge_file,
        max_snippets=limit,
        max_chars_per_snippet=max_chars_per_snippet,
        query_message_limit=query_message_limit,
        allowed_source_prefixes=allowed_source_prefixes,
    )


def summarize_retrieved_contexts(
    retrieved_contexts: list[RetrievedContext],
) -> list[dict[str, Any]]:
    return [item.to_payload_source() for item in retrieved_contexts]
