# FastFiles 局域网文件传输工具 Spec

## Why
在局域网环境中，用户需要一个轻量、零配置的文件共享工具——只需选择文件即可生成下载链接，其他设备通过浏览器即可下载，无需双方安装任何额外软件。

## What Changes
- 创建一个基于 Tauri v2 + React + TypeScript + MUI 的桌面应用
- Rust 后端内嵌 HTTP 服务器，对外提供文件下载和下载页面
- 前端提供文件选择、共享列表管理、链接复制功能
- 自动检测本机局域网 IP，使用随机端口避免冲突
- 应用关闭后所有共享数据自动清空，无需持久化

## Impact
- Affected specs: 全新项目，无现有 specs
- Affected code: 全新项目，项目根目录 `d:\Rust\FastFiles`

---

## ADDED Requirements

### Requirement: 项目初始化
系统 SHALL 使用 Tauri v2 初始化项目，前端采用 React + TypeScript + MUI，Rust 后端使用 actix-web 作为 HTTP 服务器框架。

#### Scenario: 项目结构正确
- **WHEN** 项目初始化完成
- **THEN** 项目包含 `src-tauri/`（Rust 后端）和 `src/`（React 前端）目录，`Cargo.toml` 包含 tauri 和 actix-web 依赖，`package.json` 包含 react、@mui/material 依赖

---

### Requirement: 文件选择与共享
系统 SHALL 提供「传输文件」按钮，点击后调用系统原生文件选择对话框，用户选择一个或多个文件后，系统生成唯一下载 token 并启动/复用 HTTP 服务器。

#### Scenario: 选择文件并生成链接
- **WHEN** 用户点击「传输文件」按钮并选择一个文件
- **THEN** 文件出现在共享文件列表中，显示文件名、文件大小、下载链接和操作按钮

#### Scenario: 选择多个文件
- **WHEN** 用户使用文件选择对话框选择多个文件
- **THEN** 每个文件独立出现在共享文件列表中，各自拥有独立的下载链接

#### Scenario: HTTP 服务器启动
- **WHEN** 第一个文件被添加到共享列表
- **THEN** HTTP 服务器自动启动在随机可用端口上

#### Scenario: HTTP 服务器复用
- **WHEN** 已有共享文件且 HTTP 服务器已在运行
- **THEN** 新添加文件复用现有 HTTP 服务器，不启动新端口

---

### Requirement: 局域网 IP 自动检测
系统 SHALL 自动检测本机局域网 IPv4 地址，用于生成下载链接。若无法检测到局域网 IP，回退为 127.0.0.1 并提示用户。

#### Scenario: 检测到局域网 IP
- **WHEN** 本机存在有效的非回环 IPv4 地址
- **THEN** 下载链接使用该 IP 地址，格式为 `http://{ip}:{port}/download/{token}`

#### Scenario: 未检测到局域网 IP
- **WHEN** 本机无有效的非回环 IPv4 地址
- **THEN** 下载链接使用 `127.0.0.1`，前端显示警告提示用户检查网络

---

### Requirement: 随机端口分配
系统 SHALL 使用随机可用端口启动 HTTP 服务器，端口冲突时自动重试，最多尝试 10 次。

#### Scenario: 端口正常分配
- **WHEN** 系统需要启动 HTTP 服务器且随机端口未被占用
- **THEN** 服务器在该端口上成功启动

#### Scenario: 端口冲突重试
- **WHEN** 随机选中的端口已被占用
- **THEN** 系统自动尝试另一个随机端口，最多重试 10 次

#### Scenario: 所有端口尝试失败
- **WHEN** 连续 10 次端口尝试均被占用
- **THEN** 系统向前端返回错误信息，提示用户端口不可用

---

### Requirement: HTTP 下载服务
系统 SHALL 提供两个 HTTP 端点：`/download/{token}` 返回下载页面（HTML），`/api/download/{token}` 流式返回文件二进制数据。

#### Scenario: 下载页面展示
- **WHEN** 浏览器访问 `/download/{token}` 且 token 有效
- **THEN** 返回一个包含文件名、文件大小（格式化显示 KB/MB/GB）和下载按钮的简洁 HTML 页面

#### Scenario: 文件流式下载
- **WHEN** 用户在下载页面点击下载按钮或直接访问 `/api/download/{token}`
- **THEN** 浏览器以流式方式接收文件数据，Content-Disposition 设置为 attachment，触发浏览器下载

#### Scenario: 大文件下载无内存压力
- **WHEN** 下载一个超大文件（如 4GB）
- **THEN** 系统使用流式传输，不会将整个文件加载到内存中

#### Scenario: Token 不存在
- **WHEN** 访问的 token 不存在或已被删除
- **THEN** 返回 404 状态码和友好的错误提示页面

#### Scenario: 原文件被移动或删除
- **WHEN** 共享的文件在磁盘上被移动或删除后，有人尝试下载
- **THEN** 返回 404 状态码，提示文件不存在

---

### Requirement: 共享文件列表管理
系统 SHALL 在主页面展示共享文件列表，每项显示文件名、文件大小、下载链接、复制按钮和删除按钮。

#### Scenario: 复制下载链接
- **WHEN** 用户点击某个共享文件的复制按钮
- **THEN** 下载链接被复制到剪贴板，前端显示复制成功提示

#### Scenario: 删除共享文件
- **WHEN** 用户点击某个共享文件的删除按钮
- **THEN** 该文件从共享列表移除，对应的 token 立即失效，后续访问返回 404

#### Scenario: 删除最后一个文件后服务器关闭
- **WHEN** 用户删除了共享列表中最后一个文件
- **THEN** HTTP 服务器自动关闭

---

### Requirement: 数据生命周期
系统 SHALL 在应用关闭时清空所有共享数据（token、文件映射、HTTP 服务器），不进行任何持久化存储。每次启动均为全新空白状态。

#### Scenario: 应用关闭清空数据
- **WHEN** 用户关闭应用窗口
- **THEN** HTTP 服务器停止运行，所有 token 和文件映射被释放

#### Scenario: 应用重启后空白状态
- **WHEN** 用户重新启动应用
- **THEN** 共享文件列表为空，HTTP 服务器未运行

---

### Requirement: 错误处理与边界情况
系统 SHALL 妥善处理网络异常、端口冲突、文件变更等边界情况。

#### Scenario: 重复添加同一文件
- **WHEN** 用户再次选择已共享过的文件
- **THEN** 系统为该文件生成新的独立 token 和链接（不合并、不覆盖）

#### Scenario: 网络断开
- **WHEN** 本机网络断开
- **THEN** 前端显示提示，但本地回环地址（127.0.0.1）仍可用于本机测试下载

#### Scenario: 多网卡环境
- **WHEN** 本机存在多个网络接口
- **THEN** 系统自动选择第一个检测到的非回环 IPv4 地址
