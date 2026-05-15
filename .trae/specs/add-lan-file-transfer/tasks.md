# Tasks

- [x] Task 1: 项目初始化 — 使用 Tauri v2 创建项目，配置 React + TypeScript + MUI 前端和 Rust 后端
  - [x] Subtask 1.1: 使用 `npm create tauri-app@latest` 初始化项目，选择 React + TypeScript 模板
  - [x] Subtask 1.2: 安装 MUI 依赖（@mui/material @emotion/react @emotion/styled @mui/icons-material）
  - [x] Subtask 1.3: 在 `src-tauri/Cargo.toml` 中添加 actix-web、uuid、tokio 等 Rust 依赖
  - [x] Subtask 1.4: 配置 Tauri 窗口设置（标题、默认尺寸 800×600）
  - [x] Subtask 1.5: 清理模板代码，准备空白工作区

- [x] Task 2: Rust 后端 — 实现局域网 IP 自动检测模块
  - [x] Subtask 2.1: 创建 `src-tauri/src/network.rs`，实现 `detect_lan_ip()` 函数
  - [x] Subtask 2.2: 遍历网络接口，返回第一个非回环 IPv4 地址
  - [x] Subtask 2.3: 处理多网卡和无网络场景（回退 127.0.0.1）

- [x] Task 3: Rust 后端 — 实现文件管理与 token 生成模块
  - [x] Subtask 3.1: 创建 `src-tauri/src/file_manager.rs`，定义 `SharedFile` 结构体和 `FileManager` 管理器
  - [x] Subtask 3.2: 实现 `add_file(path)` — 生成 UUID token，存储文件路径映射
  - [x] Subtask 3.3: 实现 `remove_file(token)` — 移除 token 映射
  - [x] Subtask 3.4: 实现 `get_file(token)` — 根据 token 查询文件信息
  - [x] Subtask 3.5: 实现 `list_files()` — 返回所有共享文件列表
  - [x] Subtask 3.6: 使用 `Arc<Mutex<HashMap>>` 保证线程安全

- [x] Task 4: Rust 后端 — 实现 HTTP 服务器模块
  - [x] Subtask 4.1: 创建 `src-tauri/src/http_server.rs`，基于 actix-web 构建
  - [x] Subtask 4.2: 实现 `/download/{token}` 端点 — 返回 HTML 下载页面（文件名 + 格式化大小 + 下载按钮）
  - [x] Subtask 4.3: 实现 `/api/download/{token}` 端点 — 流式传输文件，设置 Content-Disposition
  - [x] Subtask 4.4: 实现 404 错误页面（token 不存在 / 文件已删除场景）
  - [x] Subtask 4.5: 文件大小格式化辅助函数（B/KB/MB/GB）
  - [x] Subtask 4.6: 实现随机端口分配与重试逻辑（最多 10 次）

- [x] Task 5: Rust 后端 — 实现 Tauri Commands 命令层
  - [x] Subtask 5.1: 创建 `src-tauri/src/commands.rs`，使用 Tauri State 管理全局状态
  - [x] Subtask 5.2: 实现 `share_file` 命令 — 打开文件对话框、添加文件、返回文件信息
  - [x] Subtask 5.3: 实现 `get_shared_files` 命令 — 返回文件列表
  - [x] Subtask 5.4: 实现 `delete_shared_file` 命令 — 删除指定文件
  - [x] Subtask 5.5: 实现 `get_server_info` 命令 — 返回当前服务器基础 URL
  - [x] Subtask 5.6: 在 `main.rs` 中注册所有命令，初始化全局状态

- [x] Task 6: React 前端 — 实现页面布局与「传输文件」按钮
  - [x] Subtask 6.1: 创建 `src/App.tsx`，使用 MUI 主题和布局组件
  - [x] Subtask 6.2: 实现顶部标题栏（应用名称 + 图标）
  - [x] Subtask 6.3: 实现「传输文件」按钮（使用 MUI Button），点击调用 `share_file` 命令
  - [x] Subtask 6.4: 按钮添加 loading 状态和错误提示

- [x] Task 7: React 前端 — 实现共享文件列表组件
  - [x] Subtask 7.1: 创建 `src/components/SharedFileList.tsx`，使用 MUI List 组件
  - [x] Subtask 7.2: 每项展示文件名、格式化文件大小、下载链接、复制按钮、删除按钮
  - [x] Subtask 7.3: 复制按钮调用剪贴板 API，使用 MUI Snackbar 显示"已复制"提示
  - [x] Subtask 7.4: 删除按钮弹出确认对话框（MUI Dialog），确认后调用 `delete_shared_file`
  - [x] Subtask 7.5: 空列表时显示占位提示文字

- [x] Task 8: 集成测试与调试
  - [x] Subtask 8.1: 验证 `cargo check` 编译通过，无类型错误
  - [x] Subtask 8.2: 验证 `npm run build` 前端构建通过
  - [x] Subtask 8.3: 启动应用，验证完整流程：选择文件 → 生成链接 → 浏览器下载 → 删除文件

# Task Dependencies
- Task 2、Task 3 独立，可并行开发
- Task 4 依赖 Task 3（HTTP 服务器需要文件管理器）
- Task 5 依赖 Task 2、Task 3、Task 4（命令层聚合所有后端能力）
- Task 6、Task 7 依赖 Task 5（前端需要通过命令与后端交互）
- Task 8 依赖全部 Task 完成
