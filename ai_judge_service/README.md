# AI Judge Service

用于接收 `chat_server` 的评审派发请求，执行双次评估逻辑（`mock` 或 `openai`），并回调内部接口写入评审结果。

## 目录结构

- `app/main.py`: FastAPI 入口与回调编排
- `app/models.py`: 派发请求与回调模型
- `app/scoring_core.py`: 不依赖三方框架的评分核心
- `app/scoring.py`: `pydantic` 适配层
- `app/runtime_policy.py`: provider 与环境开关解析
- `app/openai_judge.py`: OpenAI 运行时（双次评估 + 合并）
- `tests/test_scoring_core.py`: 评分核心单测（`unittest`）

## 快速启动

```bash
cd ai_judge_service
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app.main:app --host 0.0.0.0 --port 8787
```

## 环境变量

- `AI_JUDGE_INTERNAL_KEY`: 与 `chat_server.ai_judge.internal_key` 保持一致
- `CHAT_SERVER_BASE_URL`: 例如 `http://127.0.0.1:6688`
- `CHAT_SERVER_REPORT_PATH_TEMPLATE`: 默认 `/api/internal/ai/judge/jobs/{job_id}/report`
- `CHAT_SERVER_FAILED_PATH_TEMPLATE`: 默认 `/api/internal/ai/judge/jobs/{job_id}/failed`
- `CALLBACK_TIMEOUT_SECONDS`: 回调超时，默认 `8`
- `JUDGE_PROCESS_DELAY_MS`: 模拟处理耗时，默认 `0`
- `JUDGE_STYLE_MODE`: 系统级文风开关，`rational|entertaining|mixed`，默认 `rational`
- `AI_JUDGE_PROVIDER`: `mock|openai`，默认 `mock`
- `OPENAI_API_KEY`: 当 `AI_JUDGE_PROVIDER=openai` 时必填
- `AI_JUDGE_OPENAI_MODEL`: 默认 `gpt-4.1-mini`
- `AI_JUDGE_OPENAI_BASE_URL`: 默认 `https://api.openai.com/v1`
- `AI_JUDGE_OPENAI_TIMEOUT_SECONDS`: OpenAI 请求超时，默认 `25`
- `AI_JUDGE_OPENAI_TEMPERATURE`: 默认 `0.1`
- `AI_JUDGE_OPENAI_MAX_RETRIES`: 每次评估重试次数，默认 `2`
- `AI_JUDGE_OPENAI_FALLBACK_TO_MOCK`: OpenAI 失败时是否回退到 mock，默认 `true`

## 运行测试

依赖安装完成后执行：

```bash
cd ai_judge_service
python3 -m unittest discover -s tests -p "test_*.py" -v
```
