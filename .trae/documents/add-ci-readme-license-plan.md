# 添加 GitHub CI/CD、README 和 Apache 2.0 许可证 - 实施计划

## 项目背景

**FastFiles** 是一个基于 Tauri v2 的局域网文件传输桌面应用，使用 React + TypeScript 前端和 Rust 后端。当前项目功能完整但缺少：
- CI/CD 自动发布流程
- 项目说明文档（README）
- 开源许可证

---

## 任务 1：创建 GitHub CI/CD 工作流文件

### 目标
创建 `.github/workflows/release.yml`，实现当推送版本标签（如 `v0.3.0`）时自动构建多平台 Tauri 应用并发布到 GitHub Release。

### 实施步骤

1. **创建目录结构**
   - 创建 `.github/workflows/` 目录

2. **编写 `release.yml` 工作流文件**，包含以下内容：

   **触发器：**
   - `push` 匹配 `v*` 标签（如 `v0.3.0`）
   - `workflow_dispatch` 手动触发

   **Job 1: `create-release`**
   - 运行环境：`ubuntu-latest`
   - 步骤：
     - checkout 代码
     - 安装 Node.js（从 `.nvmrc` 或 `package.json` engine 字段检测版本，默认 22）
     - 安装 Rust 工具链（stable）
     - 安装前端依赖（`npm ci`）
     - 构建前端（`npm run build`）
     - 使用 `tauri-apps/tauri-action@v0` 构建并发布 Release
       - 配置 `releaseId` 从创建的 Release 获取
       - 配置 `GITHUB_TOKEN` secret
       - 配置 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（可选，用于代码签名）

   **Job 2: `build-and-release`**
   - 策略矩阵：`os: [ubuntu-latest, windows-latest, macos-latest]`
   - 步骤：
     - checkout 代码
     - 安装 Node.js 22
     - 安装 Rust（stable）
     - 安装系统依赖（Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev` 等 Tauri 依赖）
     - 安装前端依赖
     - 使用 `tauri-apps/tauri-action@v0` 构建
       - 配置 `releaseId` 引用上一步创建的 release
       - 上传各平台构建产物

   **构建产物：**
   - Windows: `.msi` 安装包 和 `.exe` 安装包
   - macOS: `.dmg` 磁盘镜像
   - Linux: `.deb` 和 `.AppImage`

3. **标签触发流程说明**
   - 开发者执行 `git tag v0.3.0 && git push origin v0.3.0` 即可触发发布

---

## 任务 2：创建 Apache 2.0 许可证文件

### 目标
在项目根目录创建 `LICENSE` 文件，使用完整的 Apache License 2.0 文本。

### 实施步骤

1. **创建 `LICENSE` 文件**
   - 使用 Apache License 2.0 标准模板
   - 版权年份：2026
   - 版权持有人：FastFiles Contributors

---

## 任务 3：创建 README.md（中英双语）

### 目标
创建项目根目录的 `README.md`，包含中文和英文双语说明。

### 实施步骤

1. **创建 `README.md`** 文件，包含以下章节（中英双语）：

   **中文部分（前半）：**
   - 项目名称与徽章（GitHub Actions 状态徽章、License 徽章）
   - 项目简介：局域网零配置文件传输工具
   - 功能特性：
     - 🚀 零拷贝文件传输（Linux sendfile / Windows TransmitFile）
     - 📡 自动检测局域网 IP
     - 🔗 生成可分享的下载链接（含精美下载页面）
     - 📊 HTTP Range 断点续传支持
     - 🖥️ 跨平台支持（Windows / macOS / Linux）
     - 🔒 安全：UUID Token 访问控制，应用关闭即失效
   - 技术栈：Tauri v2 + React + TypeScript + Rust + actix-web
   - 系统要求（各平台依赖）
   - 安装说明
   - 开发指南（克隆、安装依赖、运行、构建）
   - 使用方法（图文步骤）
   - 项目结构
   - 许可证

   **英文部分（后半）：**
   - 与中文内容对应的英文翻译
   - 保持相同的结构和格式

2. **添加徽章**
   - GitHub Actions 工作流状态徽章
   - License 徽章（Apache 2.0）
   - Tauri v2 徽章
   - Rust 徽章

---

## 任务清单

1. [ ] 创建 `.github/workflows/release.yml` CI/CD 工作流文件
2. [ ] 创建 `LICENSE` Apache 2.0 许可证文件
3. [ ] 创建 `README.md` 中英双语说明文档
