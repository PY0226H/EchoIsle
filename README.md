# AiComm

在线辩论 + AI 裁判平台（MVP 阶段）。

当前仓库包含：
1. Rust 后端（聊天/辩论/支付/通知/分析）
2. Vue + Tauri 前端
3. Python AI 裁判服务（FastAPI，多 Agent + RAG）

## 当前状态（2026-02）

已具备的主链路：
1. 注册登录、工作区聊天、文件上传下载
2. Debate Lobby / Debate Room、实时消息、置顶消息
3. 钱包与 IAP 验单后端链路（含 mock 能力）
4. AI 裁判任务派发、报告回写、平局投票与二番战
5. Ops 后台：辩题/场次管理

仍未完成的生产化项：
1. iOS 真机 StoreKit 交易闭环（当前仍有 mock 桥接）
2. 生产配置收口（支付/AI mock 策略）
3. 生产环境部署与合规、监控、告警

## 技术栈

1. `chat/chat_server`：Rust + Axum + SQLx + PostgreSQL
2. `chat/notify_server`：Rust + Axum + SSE/WebSocket + LISTEN/NOTIFY
3. `chat/analytics_server`：Rust + Protobuf analytics 聚合
4. `chatapp`：Vue 3 + Vite + Tauri
5. `ai_judge_service`：Python + FastAPI（mock/openai，file/milvus RAG）

## 目录结构

```text
aicomm/
├── chat/                    # Rust 多服务后端
├── chatapp/                 # Vue + Tauri 前端
├── ai_judge_service/        # Python AI 裁判服务
├── docs/                    # 产品计划、讲解、压测结果
├── start.sh                 # 一键启动（本地开发）
└── stop.sh                  # 一键停止（本地开发）
```

## 本地快速启动

## 1) 前置依赖

1. Node.js >= 18
2. Rust stable
3. PostgreSQL >= 14
4. Yarn
5. macOS 下建议安装 Homebrew（`start.sh` 使用 `brew services`）

## 2) 数据库配置

根目录 `.env` 示例见：
- `/Users/panyihang/Documents/aicomm/.env.example`

默认本地库名是 `chat`，连接示例：
```env
DATABASE_URL=postgres://<username>:<password>@localhost:5432/chat
```

## 3) 一键启动（推荐）

```bash
./start.sh
```

脚本会自动：
1. 检查并启动 PostgreSQL（macOS）
2. 检查 `chat` 数据库
3. 执行运行时核心表修复脚本
4. 启动 `chat_server`、`notify_server`、`chatapp`

启动后访问：
- `http://localhost:1420`

停止服务：
```bash
./stop.sh
```

## 4) 手动启动

详见：
- `/Users/panyihang/Documents/aicomm/START_GUIDE.md`

## 可选：启动 AI 裁判服务

```bash
cd ai_judge_service
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python -m uvicorn app.main:app --host 0.0.0.0 --port 8787
```

注意：项目内 Python 任务统一使用虚拟环境解释器，避免全局 Python 污染。

## 测试与质量门禁

Rust 全量门禁（fmt/check/clippy/nextest）：
```bash
bash skills/post-module-test-guard/scripts/run_test_gate.sh --mode full
```

AI 服务单测：
```bash
cd ai_judge_service
.venv/bin/python -m unittest discover -s tests -p "test_*.py" -v
```

## 压测文档

1. 压测方案 v1：
- `/Users/panyihang/Documents/aicomm/docs/压测方案-v1.md`
2. 本机基线压测结果：
- `/Users/panyihang/Documents/aicomm/docs/压测结果-v1-2026-02-27.md`
3. 本机极限探索结果：
- `/Users/panyihang/Documents/aicomm/docs/压测结果-v2-本机极限探索-2026-02-27.md`

## 服务端口

| Service | Port | 说明 |
|---|---:|---|
| chat_server | 6688 | 主业务 API |
| notify_server | 6687 | SSE / WebSocket 推送 |
| chatapp (vite) | 1420 | Web 前端 |
| ai_judge_service | 8787 | AI 裁判内部服务（可选） |

## 生产部署注意事项

1. 外网必须使用 `HTTPS/WSS`，不要直接暴露 `HTTP/WS`。
2. 生产环境禁止使用默认 mock 配置（支付、AI provider/fallback）。
3. 不要在生产中使用开发密钥与示例配置。
4. 上线前必须在云预发环境完成容量压测，不以本机压测结果做容量承诺。
