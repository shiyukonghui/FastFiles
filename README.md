# FastFiles

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev)

🚀 **局域网零配置文件传输工具** —— 选择文件，生成链接，局域网内任意设备即可高速下载。

---

## 📑 目录

- [中文文档](#中文文档)
  - [功能特性](#功能特性)
  - [技术栈](#技术栈)
  - [系统要求](#系统要求)
  - [安装](#安装)
  - [开发指南](#开发指南)
  - [使用方法](#使用方法)
  - [项目结构](#项目结构)
  - [许可证](#许可证)
- [English Documentation](#english-documentation)
  - [Features](#features)
  - [Tech Stack](#tech-stack)
  - [System Requirements](#system-requirements)
  - [Installation](#installation)
  - [Development Guide](#development-guide)
  - [Usage](#usage)
  - [Project Structure](#project-structure)
  - [License](#license-1)

---

## 中文文档

### 功能特性

| 特性 | 说明 |
|------|------|
| 🚀 **零拷贝传输** | Linux 使用 `sendfile`、Windows 使用 `TransmitFile`，系统级零拷贝，最大化吞吐量 |
| 📡 **自动 IP 检测** | 自动检测本机所有局域网 IP 地址，无需手动配置 |
| 🔗 **可分享链接** | 每个共享文件生成唯一 UUID Token，提供精美的下载页面 |
| 📊 **断点续传** | 支持 HTTP Range 请求，大文件下载中断后可续传 |
| 🖥️ **跨平台** | 支持 Windows、macOS、Linux 三大桌面平台 |
| 🔒 **安全设计** | UUID Token 访问控制，应用关闭即清除所有共享记录 |
| ⚡ **极速性能** | mmap 内存映射 + TCP 底层优化，最大化局域网带宽利用率 |

### 技术栈

| 层 | 技术 |
|----|------|
| **桌面框架** | [Tauri v2](https://tauri.app) |
| **前端** | React 19 + TypeScript 5.5 + Vite 6 |
| **UI 库** | MUI 6 (Material-UI) |
| **后端语言** | Rust (edition 2021) |
| **HTTP 服务** | actix-web 4 + tokio 异步运行时 |
| **文件传输** | memmap2 内存映射 + 操作系统级零拷贝 |

### 系统要求

**Windows:**
- Windows 10 及以上
- [Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)（Windows 10 通常已预装）

**macOS:**
- macOS 11 Big Sur 及以上

**Linux:**
- 需要以下系统库：
  ```bash
  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
    libappindicator3-dev librsvg2-dev libssl-dev \
    libjavascriptcoregtk-4.1-dev libsoup-3.0-dev
  ```

### 安装

从 [GitHub Releases](https://github.com/shiyukonghui/FastFiles/releases) 下载对应平台的安装包：

- **Windows**: `.msi` 安装程序或 `.exe` 安装包
- **macOS**: `.dmg` 磁盘镜像
- **Linux**: `.deb` 安装包或 `.AppImage` 便携版

### 开发指南

```bash
# 1. 克隆项目
git clone https://github.com/shiyukonghui/FastFiles.git
cd FastFiles

# 2. 安装前端依赖
npm install

# 3. 启动开发模式（同时启动前端热重载和 Tauri 窗口）
npm run tauri dev

# 4. 构建生产版本
npm run tauri build
```

> **注意**：开发模式下，首次启动会自动下载 Rust 依赖并编译后端代码，可能需要几分钟时间。

### 使用方法

1. 启动 FastFiles 应用
2. 点击 **"传输文件"** 按钮，选择要分享的文件
3. 应用自动生成下载链接，展示所有局域网 IP 对应的 URL
4. 点击 **"复制"** 按钮复制链接，发送给局域网内的接收者
5. 接收者在浏览器中打开链接，即可看到精美下载页面并下载文件
6. 不需要时，点击 **"删除"** 按钮移除共享文件

> **提示**：关闭 FastFiles 应用后，所有共享文件将自动清除，链接即刻失效，确保数据安全。

### 项目结构

```
FastFiles/
├── src/                        # React 前端源码
│   ├── App.tsx                 # 主应用组件
│   ├── main.tsx                # React 入口
│   └── components/
│       └── SharedFileList.tsx  # 共享文件列表组件
├── src-tauri/                  # Tauri Rust 后端
│   ├── Cargo.toml              # Rust 依赖配置
│   ├── tauri.conf.json         # Tauri 应用配置
│   └── src/
│       ├── main.rs             # 程序入口
│       ├── lib.rs              # 应用构建器
│       ├── commands.rs         # Tauri IPC 命令
│       ├── http_server.rs      # HTTP 下载服务器
│       ├── file_manager.rs     # 共享文件管理器
│       ├── network.rs          # 局域网 IP 检测
│       └── sendfile.rs         # 零拷贝传输工具
├── package.json                # 前端依赖和脚本
├── vite.config.ts              # Vite 构建配置
└── tsconfig.json               # TypeScript 配置
```

### 许可证

本项目采用 [Apache License 2.0](LICENSE) 许可证。

---

## English Documentation

### Features

| Feature | Description |
|---------|-------------|
| 🚀 **Zero-Copy Transfer** | Uses `sendfile` on Linux and `TransmitFile` on Windows for OS-level zero-copy, maximizing throughput |
| 📡 **Auto IP Detection** | Automatically detects all LAN IP addresses, no manual configuration needed |
| 🔗 **Shareable Links** | Generates unique UUID tokens for each shared file, with an elegant download page |
| 📊 **Resumable Downloads** | Supports HTTP Range requests for resuming interrupted large file downloads |
| 🖥️ **Cross-Platform** | Supports Windows, macOS, and Linux |
| 🔒 **Secure by Design** | UUID token access control; all shared records are cleared when the app closes |
| ⚡ **Blazing Fast** | mmap memory mapping + low-level TCP optimization for maximum LAN bandwidth utilization |

### Tech Stack

| Layer | Technology |
|-------|------------|
| **Desktop Framework** | [Tauri v2](https://tauri.app) |
| **Frontend** | React 19 + TypeScript 5.5 + Vite 6 |
| **UI Library** | MUI 6 (Material-UI) |
| **Backend Language** | Rust (edition 2021) |
| **HTTP Server** | actix-web 4 + tokio async runtime |
| **File Transfer** | memmap2 memory mapping + OS-level zero-copy |

### System Requirements

**Windows:**
- Windows 10 or later
- [Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed on Windows 10+)

**macOS:**
- macOS 11 Big Sur or later

**Linux:**
- Required system libraries:
  ```bash
  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
    libappindicator3-dev librsvg2-dev libssl-dev \
    libjavascriptcoregtk-4.1-dev libsoup-3.0-dev
  ```

### Installation

Download the installation package for your platform from [GitHub Releases](https://github.com/shiyukonghui/FastFiles/releases):

- **Windows**: `.msi` installer or `.exe` setup
- **macOS**: `.dmg` disk image
- **Linux**: `.deb` package or `.AppImage` portable

### Development Guide

```bash
# 1. Clone the repository
git clone https://github.com/shiyukonghui/FastFiles.git
cd FastFiles

# 2. Install frontend dependencies
npm install

# 3. Start development mode (launches frontend hot-reload and Tauri window)
npm run tauri dev

# 4. Build for production
npm run tauri build
```

> **Note**: On first launch in development mode, Rust dependencies will be automatically downloaded and the backend will be compiled, which may take a few minutes.

### Usage

1. Launch the FastFiles application
2. Click the **"传输文件"** (Share File) button and select the file(s) you want to share
3. The app automatically generates download links, displaying URLs for all detected LAN IPs
4. Click the **"复制"** (Copy) button to copy a link and send it to recipients on the same LAN
5. Recipients open the link in a browser to view an elegant download page and download the file
6. Click the **"删除"** (Delete) button to remove shared files when no longer needed

> **Tip**: All shared files are automatically cleared when you close FastFiles, and all links become instantly invalid, ensuring data security.

### Project Structure

```
FastFiles/
├── src/                        # React frontend source
│   ├── App.tsx                 # Main application component
│   ├── main.tsx                # React entry point
│   └── components/
│       └── SharedFileList.tsx  # Shared file list component
├── src-tauri/                  # Tauri Rust backend
│   ├── Cargo.toml              # Rust dependency configuration
│   ├── tauri.conf.json         # Tauri application configuration
│   └── src/
│       ├── main.rs             # Program entry point
│       ├── lib.rs              # Application builder
│       ├── commands.rs         # Tauri IPC commands
│       ├── http_server.rs      # HTTP download server
│       ├── file_manager.rs     # Shared file manager
│       ├── network.rs          # LAN IP detection
│       └── sendfile.rs         # Zero-copy transfer utility
├── package.json                # Frontend dependencies and scripts
├── vite.config.ts              # Vite build configuration
└── tsconfig.json               # TypeScript configuration
```

### License

This project is licensed under the [Apache License 2.0](LICENSE).
