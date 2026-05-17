# QuickHorse 开发 Roadmap

## 版本规划总览

| 版本 | 状态 | 发布时间 | 核心目标 |
|------|------|----------|----------|
| v0.1.0 | ✅ 已完成 | 2026-05-10 | MVP - 基础 Agent 功能 |
| v0.2.0 | ✅ 已完成 | 2026-05-11 | 流式输出 + 进度指示 + 错误分类 |
| v0.3.0 | ✅ 已完成 | 2026-05-11 | 权限控制 + 安全存储 + Tab补全 |
| v0.4.0 | ✅ 已完成 | 2026-05-11 | 高级工具 + Agent增强 |
| v0.4.1 | ✅ 已完成 | 2026-05-11 | 版本整理 + 文档更新 |
| v0.4.2 | ✅ 已完成 | 2026-05-11 | ImageTool + DatabaseTool + ProviderCapabilities |
| v0.4.3 | ✅ 已完成 | 2026-05-11 | 修复 slash 命令崩溃 (no reactor running) |
| v0.4.4 | 📋 规划中 | TBD | UX改进 - 欢迎屏/高亮条/耗时彩蛋 (参考Claude Code设计) |
| v0.5.0 | ✅ 已完成 | 2026-05-15 | TUI Widget 系统 + Scroll 修复 |
| v0.5.1 | ✅ 已完成 | 2026-05-17 | Grapheme换行 + Warnings清理 |
| v0.5.2 | 🔄 进行中 | TBD | Cost追踪 + Hooks + Compaction (参考DeepSeek-TUI) |
| v0.6.0 | 📋 规划中 | TBD | 思考模式 + GitHub/Docker集成 |
| v1.0.0 | 🎯 愿景 | TBD | 完整 CLI Agent |

---

## v0.1.0 - MVP (已完成)

### 核心功能

| 模块 | 功能 | 状态 | 说明 |
|------|------|------|------|
| **CLI** | 基础命令行 | ✅ | clap 参数解析、--help、--version |
| **TUI** | 终端界面 | ✅ | ratatui 实现、输入模式、消息显示 |
| **Setup Wizard** | 首次配置 | ✅ | 交互式配置向导、API Key 设置 |
| **Slash Commands** | TUI 命令 | ✅ | /help、/provider、/model、/clear、/status、/session |

### Provider 支持

| Provider | 模型 | API Key 来源 | 状态 |
|----------|------|--------------|------|
| OpenAI | GPT-4, GPT-4o, GPT-3.5-turbo | OPENAI_API_KEY | ✅ |
| Anthropic | Claude 3.5 Sonnet, Claude 3 Opus | ANTHROPIC_API_KEY | ✅ |
| Gemini | Gemini 1.5 Pro, Gemini 1.5 Flash | GEMINI_API_KEY | ✅ |
| Ollama | Llama3, Mistral, Qwen2 | 本地 (无 Key) | ✅ |
| 兼容 API | BaiLian, DeepSeek, Moonshot | base_url 参数 | ✅ |

### 工具实现

| 工具 | 功能 | 状态 | 权限控制 |
|------|------|------|----------|
| BashTool | 执行 shell 命令 | ✅ | 基础 |
| FileReadTool | 读取文件内容 | ✅ | 只读 |
| FileEditTool | 编辑文件 (查找替换) | ✅ | 写入 |
| GlobTool | 文件模式匹配 | ✅ | 只读 |
| GrepTool | 内容搜索 (正则) | ✅ | 只读 |
| WebFetchTool | 获取网页内容 | ✅ | 网络 |

### 其他功能

| 功能 | 状态 | 说明 |
|------|------|------|
| MCP Server | ✅ | JSON-RPC 2.0 协议实现 |
| MCP Client | ✅ | 连接外部 MCP 服务器 |
| Session 管理 | ✅ | 会话持久化 (.quickhorse/sessions/) |
| 配置管理 | ✅ | ~/.quickhorse/config.toml |
| 多轮对话 | ✅ | 消息历史正确传递 |
| 动态 Provider 切换 | ✅ | Arc<RwLock<dyn Provider>> |

---

## v0.2.0 - 用户体验优化 (进行中)

### 优先级 P0 (必须)

| 功能 | 描述 | 状态 | 说明 |
|------|------|------|------|
| **流式输出** | 实时显示 LLM 响应 | ✅ 已完成 | OpenAI/Anthropic/Gemini/Ollama streaming |
| **进度指示** | 工具执行时显示进度 | ✅ 已完成 | Spinner、ProgressBar、ToolProgress tracking |
| **错误处理优化** | 用户友好错误提示 | ✅ 已完成 | ErrorCode/QuickHorseError 分类系统、TUI boxed 错误显示 |
| **日志系统** | 可调试日志输出 | ✅ 已完成 | tracing crate、--verbose/--debug 参数 |

### 优先级 P1 (重要)

| 功能 | 描述 | 状态 | 说明 |
|------|------|------|------|
| **权限控制** | 工具执行权限检查 | ✅ 已完成 | 白名单/黑名单、PermissionDialog UI |
| **配置加密** | API Key 安全存储 | ✅ 已完成 | macOS Keychain + encrypted file fallback |
| **Tab 补全** | 命令/路径自动补全 | ✅ 已完成 | CommandCompleter + PathCompleter |
| **历史记录搜索** | Ctrl+R 搜索历史 | 📋 待开发 | command history |

### 优先级 P2 (可选)

| 功能 | 描述 | 预估工时 | 说明 |
|------|------|----------|------|
| **主题定制** | 自定义配色方案 | 0.5 周 | config.toml 配置 |
| **快捷键自定义** | 用户自定义按键 | 0.5 周 | keybindings.json |
| **多语言提示词** | 中文/英文 system prompt | 0.5 周 | 配置选项 |

---

## v0.3.0 - 高级功能

### 工具扩展

| 工具 | 功能 | 预估工时 | 说明 |
|------|------|----------|------|
| **WriteTool** | 创建新文件 | 0.5 周 | 完整文件写入 |
| **PatchTool** | diff/patch 应用 | 1 周 | 代码修改补丁 |
| **ExecuteTool** | Python/Node 执行 | 1 周 | 安全沙箱执行 |
| **ImageTool** | 图片查看/分析 | 1 周 | 多模态支持 |
| **DatabaseTool** | SQLite 查询 | 0.5 周 | 数据库操作 |
| **GitTool** | Git 操作封装 | 0.5 周 | status、diff、commit |

### Agent 增强

| 功能 | 描述 | 预估工时 | 说明 |
|------|------|----------|------|
| **思考模式** | 显式 reasoning | 1 周 | Claude thinking equivalent |
| **并行工具** | 并行执行多工具 | 0.5 周 | 提高效率 |
| **工具链** | 工具组合执行 | 1 周 | 复杂任务分解 |
| **上下文压缩** | 长对话压缩 | 1 周 | 减少 token 使用 |

### 集成功能

| 功能 | 描述 | 预估工时 | 说明 |
|------|------|----------|------|
| **GitHub 集成** | PR/Issue 操作 | 1 周 | gh CLI 封装 |
| **Docker 集成** | 容器操作 | 0.5 周 | docker CLI 封装 |
| **测试运行器** | 自动运行测试 | 0.5 周 | cargo test、pytest |

---

## v1.0.0 - 完整版本愿景

### 核心能力对标

| 能力 | Claude Code | Cursor | QuickHorse v1.0 目标 |
|------|-------------|--------|---------------------|
| **多 Provider** | Anthropic only | Claude + others | ✅ 4+ Provider |
| **工具数量** | 20+ | 15+ | 15+ 核心工具 |
| **流式输出** | ✅ | ✅ | ✅ |
| **上下文管理** | ✅ 智能 | ✅ | ✅ 基础 |
| **文件编辑** | ✅ 智能 | ✅ 智能 | ✅ 基础 |
| **MCP 支持** | ✅ | ❌ | ✅ |
| **会话管理** | ✅ | ✅ | ✅ |
| **权限控制** | ✅ 完整 | ⚠️ 基础 | ✅ 完整 |
| **内存占用** | 未知 | 300MB+ | **10-50MB** |
| **部署方式** | 未知 | Electron | **单二进制** |

### 差异化优势

| 优势 | 说明 | 目标 |
|------|------|------|
| **极致轻量** | Rust 实现，10-50MB 内存 | 比 Node.js 方案省 80% 内存 |
| **零依赖部署** | musl 静态编译，单文件 | 复制即运行，无需安装 |
| **旧设备支持** | CentOS 5+, 嵌入式设备 | 覆盖 Node.js 不支持的场景 |
| **MCP 协议** | 标准工具协议 | 与 Claude Code 兼容 |
| **多 Provider** | 4+ LLM Provider | 用户自由选择 |
| **开源免费** | MIT License | 社区贡献 |

### 长期规划功能

| 功能 | 描述 | 优先级 | 预估 |
|------|------|--------|------|
| **Web UI** | 浏览器界面选项 | P3 | 2 周 |
| **VSCode 扩展** | IDE 集成 | P3 | 1 周 |
| **团队协作** | 多人共享 session | P3 | 2 周 |
| **模板系统** | 预定义任务模板 | P2 | 1 周 |
| **插件系统** | 用户自定义工具 | P2 | 2 周 |
| **智能重构** | 代码重构建议 | P3 | 3 周 |

---

## 发布里程碑

### v0.1.0 ✅ (2026-05-10)

```
✅ CLI 框架 + Provider 抽象
✅ 4 Provider 实现 (OpenAI, Anthropic, Gemini, Ollama)
✅ 6 核心工具 (Bash, Read, Edit, Glob, Grep, WebFetch)
✅ MCP Server/Client
✅ Session 管理 + 持久化
✅ Slash Commands (/help, /provider, /model, etc.)
✅ 首次配置向导
✅ 多轮对话修复
```

### v0.2.0 ✅ (2026-05-11)

```
✅ 流式输出 (OpenAI, Anthropic, Gemini, Ollama)
   - StreamEvent 类型系统
   - Provider trait stream_message_channel 方法
   - bytes_stream() 真正的流式处理
   - SSE/JSON lines 格式解析
   - TUI 实时渲染集成
✅ 进度指示
   - Spinner 动画组件
   - ProgressBar Unicode 块显示
   - ToolProgress 工具执行跟踪
   - ProgressManager 统一管理
✅ 错误处理优化
   - ErrorCode 错误代码系统 (E001-E404)
   - QuickHorseError 用户友好错误类型
   - classify_io_error/classify_reqwest_error 分类函数
   - Provider 特定错误解析
   - TUI boxed 错误显示
✅ 日志系统
   - tracing crate 日志框架
   - --verbose/--debug 参数
   - 文件日志 + 内存日志
   - Diagnostic 日志函数
✅ 权限控制
✅ 配置加密
```

### v0.3.0 ✅ (2026-05-11)

```
✅ 权限控制系统
   - Permission 模块 (whitelist/blacklist)
   - PermissionConfig 配置
   - PermissionDialog UI (AllowOnce/AllowAndSave/Deny/Cancel)
   - BashTool/FileEditTool 权限检查
   - 规则持久化
✅ SecureStorage 安全存储
   - macOS Keychain 集成
   - EncryptedFileStorage 加密文件
   - PlainTextStorage fallback (0o600)
   - API Key 不再存储在 config.toml
✅ Tab 补全系统
   - Completion 模块 + CompletionProvider trait
   - CommandCompleter (/help, /provider, /model, etc.)
   - PathCompleter (~/, /, ./, ../)
   - Completion popup UI
   - Tab/Shift+Tab 导航
✅ 测试扩展
   - 311 tests passing (+186)
```

### v0.4.0 ✅ (2026-05-11)

```
✅ WriteTool - 创建/覆写文件 (382行, 12 tests)
✅ GitTool - Git操作封装 (393行, 8 tests)
✅ PatchTool - diff补丁应用 (370行, 8 tests)
✅ ExecuteTool - Python/Node执行 (270行, 8 tests)
✅ 并行工具执行 - Agent并行执行多工具
✅ 上下文压缩 - Token计数和消息裁剪 (260行, 8 tests)
✅ 历史搜索 - 命令历史管理 (290行, 11 tests)

测试: 131 tests passing (+35 from v0.3.0)
```

### v0.5.0 ✅ (2026-05-15)

```
✅ TUI Widget 系统 (参考 DeepSeek-TUI 架构)
   - ChatWidget - 主对话显示 + scrollbar + jump-to-latest
   - HeaderWidget - Model/Provider/Workspace 信息
   - FooterWidget - Mode/Status chips + 动画帧
   - ToolCardWidget - Tool执行卡片 + expand affordance
✅ Scroll Bug 修复
   - pending_scroll_delta 延迟解析机制
   - resolve_scroll_for_render() 在渲染时应用滚动
   - TAIL_SENTINEL (usize::MAX) 底部追踪
✅ HistoryCell 枚举
   - Glyph markers: ▎ User, ● Assistant, ╎ Thinking
   - System message 隐藏
✅ TranscriptViewCache
   - Per-cell revision caching
   - O(1) 重新渲染优化
✅ StreamingState
   - 自适应 chunking policy

文件: +9 新增, +3252 行
测试: 227 tests passing (+96 from v0.4.0)
```

### v0.5.1 🔄 (2026-05-15)

```
🔄 Grapheme-based 换行 (进行中)
   - wrap_text() 使用 grapheme 迭代替代 word 迭代
   - 正确处理中文字符 (width=2)
   - 长单词在 grapheme 级别拆分
   - 4 个新增测试 (Chinese, long words, mixed)

📋 Resize 响应优化 (待完成)
   - Event::Resize 触发缓存清除
   - 重新计算所有 cell 的 lines(width)
   - 保持当前滚动位置

📋 计划功能
   - 自动换行边界检测优化
   - Emoji/特殊字符宽度处理
   - 性能基准测试

当前: 231 tests passing (+4 from v0.5.0)
```

### v1.0.0 🎯 (愿景)

```
🎯 15+ 核心工具
🎯 完整权限控制
🎯 智能上下文管理
🎯 流式输出 + 进度指示
🎯 MCP 完整兼容
🎯 GitHub/Docker 集成
🎯 测试运行器
```

---

## 开发建议

### ⭐ 优先参考 OpenClaw 源码实现

**OpenClaw 源码位置：** `~/ai-project/openclaude`

| 功能模块 | OpenClaw 实现路径 | 关键文件 |
|----------|-------------------|----------|
| **流式输出** | `src/ink/` | `render-to-screen.ts`, `render-node-to-output.ts` |
| **TUI 组件** | `src/ink/components/` | React Ink 组件库 |
| **Tool 定义** | `src/Tool.ts` | Tool interface + ToolResult |
| **BashTool** | `src/tools/BashTool/` | 安全检查 + 权限控制 |
| **进度显示** | `src/ink/components/` | Progress/Spinner 组件 |
| **Provider** | `src/utils/provider*.ts` | Provider discovery + recommendation |
| **MCP Client** | `src/services/mcp/client.ts` | MCP 协议实现 |
| **Session** | `src/assistant/` | AssistantSessionChooser |

### 下一步行动 (v0.2.0 续)

1. **流式输出** ✅ 已完成
   - ✅ StreamEvent 类型系统 (src/provider/stream.rs)
   - ✅ Provider trait 扩展 (stream_message_channel 方法)
   - ✅ OpenAI 真正的流式输出 (bytes_stream())
   - ✅ Anthropic SSE 流式处理
   - ✅ Gemini SSE 流式处理
   - ✅ Ollama JSON lines 流式处理
   - ✅ TUI 实时渲染集成

2. **用户体验** (下一步)
   - 参考 OpenClaw `src/ink/components/` 进度组件
   - 工具执行进度显示
   - 错误分类提示
   - --verbose 日志输出

3. **安全性**
   - 参考 OpenClaw `src/tools/BashTool/` 权限检查
   - API Key 加密存储
   - 工具白名单机制
   - 用户确认危险操作

### 技术债务

| 问题 | 影响 | 建议 |
|------|------|------|
| ✅ 无流式输出 | 已解决 | v0.2.0 完成 |
| 权限控制弱 | 安全风险 | v0.2.0 必须 |
| 错误提示粗糙 | 用户困惑 | v0.2.0 优化 |
| MCP 未集成到主流程 | 功能闲置 | v0.3.0 集成 |

---

## 参考资料

- [Claude Code](https://github.com/anthropics/claude-code) - Anthropic 官方 CLI Agent
- [Cursor](https://cursor.sh) - AI IDE
- [MCP Specification](https://spec.modelcontextprotocol.io/) - Model Context Protocol
- [ratatui](https://github.com/ratatui-org/ratatui) - Rust TUI 库