from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Any, Awaitable, Callable

from fastapi import FastAPI, Header, HTTPException

from .callback_client import callback_failed, callback_report
from .dispatch_controller import (
    BuildReportByRuntimeFn,
    CallbackFailedFn,
    CallbackReportFn,
    DispatchRuntimeConfig,
    SleepFn,
    process_dispatch_request,
)
from .models import JudgeDispatchRequest
from .runtime_orchestrator import build_report_by_runtime
from .settings import (
    Settings,
    build_callback_client_config,
    build_dispatch_runtime_config,
    load_settings,
)
from .wiring import build_dispatch_callbacks

BuildReportByRuntimeImpl = Callable[..., Awaitable[Any]]
LoadSettingsFn = Callable[[], Settings]


@dataclass(frozen=True)
class AppRuntime:
    settings: Settings
    dispatch_runtime_cfg: DispatchRuntimeConfig
    build_report_by_runtime_adapter: BuildReportByRuntimeFn
    callback_report_fn: CallbackReportFn
    callback_failed_fn: CallbackFailedFn
    sleep_fn: SleepFn


def require_internal_key(settings: Settings, header_value: str | None) -> None:
    if not header_value:
        raise HTTPException(status_code=401, detail="missing x-ai-internal-key")
    if header_value.strip() != settings.ai_internal_key:
        raise HTTPException(status_code=401, detail="invalid x-ai-internal-key")


def build_report_by_runtime_adapter(
    *,
    settings: Settings,
    build_report_by_runtime_fn: BuildReportByRuntimeImpl = build_report_by_runtime,
) -> BuildReportByRuntimeFn:
    async def _adapter(
        request: JudgeDispatchRequest,
        effective_style_mode: str,
        style_mode_source: str,
    ):
        return await build_report_by_runtime_fn(
            request=request,
            effective_style_mode=effective_style_mode,
            style_mode_source=style_mode_source,
            settings=settings,
        )

    return _adapter


def create_runtime(
    *,
    settings: Settings,
    build_report_by_runtime_fn: BuildReportByRuntimeImpl = build_report_by_runtime,
    callback_report_impl=callback_report,
    callback_failed_impl=callback_failed,
    sleep_fn: SleepFn = asyncio.sleep,
) -> AppRuntime:
    callback_cfg = build_callback_client_config(settings)
    callback_report_fn, callback_failed_fn = build_dispatch_callbacks(
        cfg=callback_cfg,
        callback_report_impl=callback_report_impl,
        callback_failed_impl=callback_failed_impl,
    )
    return AppRuntime(
        settings=settings,
        dispatch_runtime_cfg=build_dispatch_runtime_config(settings),
        build_report_by_runtime_adapter=build_report_by_runtime_adapter(
            settings=settings,
            build_report_by_runtime_fn=build_report_by_runtime_fn,
        ),
        callback_report_fn=callback_report_fn,
        callback_failed_fn=callback_failed_fn,
        sleep_fn=sleep_fn,
    )


def create_app(runtime: AppRuntime) -> FastAPI:
    app = FastAPI(title="AI Judge Service", version="0.2.0")

    @app.get("/healthz")
    async def healthz() -> dict[str, bool]:
        return {"ok": True}

    @app.post("/internal/judge/dispatch")
    async def dispatch_judge_job(
        request: JudgeDispatchRequest,
        x_ai_internal_key: str | None = Header(default=None),
    ) -> dict:
        require_internal_key(runtime.settings, x_ai_internal_key)
        return await process_dispatch_request(
            request=request,
            runtime_cfg=runtime.dispatch_runtime_cfg,
            build_report_by_runtime=runtime.build_report_by_runtime_adapter,
            callback_report=runtime.callback_report_fn,
            callback_failed=runtime.callback_failed_fn,
            sleep_fn=runtime.sleep_fn,
        )

    return app


def create_default_app(*, load_settings_fn: LoadSettingsFn = load_settings) -> FastAPI:
    return create_app(
        create_runtime(
            settings=load_settings_fn(),
        )
    )
