# 最大化文件下载性能 Spec

## Why
当前下载架构采用线程池读取文件 → 用户态内存缓冲区 → mpsc 通道 → actix-web 流式响应的多层数据拷贝路径，每次下载涉及至少 2 次用户态内存拷贝（磁盘→堆缓冲区→Bytes→通道），对超大文件（4GB+）和局域网千兆/万兆带宽场景构成严重性能瓶颈。本规范的目标是**抛弃所有现有实现思维**，以操作系统内核级零拷贝技术为核心，实现局域网内接近磁盘读取极限（或网卡带宽极限）的文件传输性能。

## What Changes
- **BREAKING**: 完全重写 HTTP 下载数据路径，用 OS 级 `sendfile`/`TransmitFile` 零拷贝替代当前的分块读取+通道传输方案
- 添加 HTTP Range 请求支持，允许客户端并行分段下载同一文件
- TCP 层面的深度优化：NODELAY、更大的 socket buffer、连接复用
- 用 `tokio::fs` 异步文件操作替代 `std::thread::spawn` 阻塞读
- 消除中间 Bytes 分配和 mpsc 通道瓶颈，实现文件描述符到 socket 的直接传输
- 添加操作系统缓存提示（`fadvise`），优化内核页缓存策略
- 前端增加并行下载速率展示（可选增强）

## Impact
- Affected specs: `add-lan-file-transfer`（HTTP 下载端点和流式下载逻辑被重写）
- Affected code: `src-tauri/src/http_server.rs`（核心重写）、`src-tauri/Cargo.toml`（新增依赖）、前端可选项

---

## ADDED Requirements

### Requirement: 操作系统零拷贝文件传输
系统 SHALL 在 Linux 上使用 `sendfile()` 系统调用、在 Windows 上使用 `TransmitFile()` API（或等效机制），实现从文件描述符到 TCP socket 的内核态直接传输，消除用户态内存拷贝。

#### Scenario: Linux sendfile 零拷贝下载
- **WHEN** 运行于 Linux 平台且客户端请求 `/api/download/{token}`
- **THEN** 文件数据通过 `sendfile()` 从内核页缓存直接传输到 socket，不经过用户态缓冲区

#### Scenario: Windows TransmitFile 零拷贝下载
- **WHEN** 运行于 Windows 平台且客户端请求 `/api/download/{token}`
- **THEN** 文件数据通过 `TransmitFile()` 或等效机制从内核直接传输

#### Scenario: 大文件零拷贝不增加内存开销
- **WHEN** 下载任意大小文件（包括 10GB+）
- **THEN** 服务端内存占用增量 < 1MB（不受文件大小影响），CPU 用户态占用极低

---

### Requirement: HTTP Range 分段下载支持
系统 SHALL 支持 HTTP Range 请求头，允许客户端指定字节范围，实现多线程/多连接并行下载同一文件。SHALL 正确响应 `Accept-Ranges: bytes` 头。

#### Scenario: 单段 Range 请求
- **WHEN** 客户端发送 `Range: bytes=0-1048575`（请求前 1MB）
- **THEN** 返回 206 Partial Content，Content-Range 为 `bytes 0-1048575/{total}`，仅传输指定范围数据

#### Scenario: 多段 Range 请求
- **WHEN** 客户端发送 `Range: bytes=0-1023,2048-4095`
- **THEN** 返回 206 Partial Content，使用 `multipart/byteranges` MIME 类型，包含两个分段

#### Scenario: 无 Range 时的完整下载
- **WHEN** 客户端未发送 Range 请求头
- **THEN** 正常返回 200 OK，Content-Length 为完整文件大小，同时响应头包含 `Accept-Ranges: bytes`

#### Scenario: Range 超出范围
- **WHEN** 客户端发送 `Range: bytes=999999999-` 且文件小于该偏移
- **THEN** 返回 416 Range Not Satisfiable，Content-Range 为 `bytes */{total}`

---

### Requirement: TCP 传输层深度优化
系统 SHALL 在所有平台配置最优 TCP 参数以最大化局域网文件传输吞吐量。

#### Scenario: TCP_NODELAY 禁用 Nagle 算法
- **WHEN** HTTP 服务器启动并接受连接
- **THEN** 所有连接 socket 设置 TCP_NODELAY，禁用 Nagle 算法，消除小包延迟

#### Scenario: Socket 缓冲区最大化
- **WHEN** HTTP 服务器创建 TCP 监听 socket
- **THEN** 发送和接收缓冲区设置为操作系统允许的最大值（不低于 8MB），以支持高带宽延迟积

#### Scenario: Keep-Alive 连接复用
- **WHEN** 客户端请求包含 `Connection: keep-alive`
- **THEN** TCP 连接在请求完成后保持打开至少 60 秒，供后续请求复用

---

### Requirement: 异步文件 I/O
系统 SHALL 使用 `tokio::fs` 异步文件操作与 `tokio::spawn_blocking`（sendfile 路径）替代当前 `std::thread::spawn`，以避免每次下载创建系统线程的开销。

#### Scenario: 并发下载不创建大量线程
- **WHEN** 10 个客户端同时下载不同文件
- **THEN** 系统不因此创建 10 个独立的 OS 线程，所有下载在 tokio 异步运行时内高效调度

---

### Requirement: 下载页面增强
下载页面 SHALL 显示更丰富的文件信息和下载速度提示。

#### Scenario: 下载页面展示增强信息
- **WHEN** 浏览器访问 `/download/{token}`
- **THEN** 页面除文件名和大小外，展示文件类型图标、下载提示和支持断点续传/Range 的说明

---

## MODIFIED Requirements

### Requirement: HTTP 下载服务（原 `add-lan-file-transfer` 规范）
系统 SHALL 提供两个 HTTP 端点：`/download/{token}` 返回下载页面（HTML），`/api/download/{token}` 传输文件二进制数据。

#### Scenario: 文件流式下载
- **WHEN** 用户在下载页面点击下载按钮或直接访问 `/api/download/{token}`
- **THEN** 数据通过操作系统零拷贝机制直接传输到 socket，Content-Disposition 设置为 attachment

#### Scenario: 大文件下载无内存压力
- **WHEN** 下载一个超大文件（如 10GB）
- **THEN** 服务端内存占用不随文件大小增长而增加（零拷贝路径确保增量 < 1MB）

#### Scenario: 并发下载性能不退化
- **WHEN** 多个客户端同时下载大文件
- **THEN** 每个客户端的下载速率接近独立下载（受限于磁盘 I/O 和网络带宽上限）

---

### Requirement: 随机端口分配（保留不变）
沿用原有端口分配逻辑，增加 socket2 参数优化。

---

## REMOVED Requirements

无移除的需求。原有功能（下载页面、token 校验、404 处理、服务器生命周期等）全部保留，仅数据路径重写。

---

## 非功能性需求

### 性能目标
- **吞吐量**: 千兆局域网（1Gbps）环境下，单客户端下载速率 ≥ 900Mbps（磁盘 IO 允许的话）
- **内存**: 服务端每连接额外内存占用 < 100KB
- **CPU**: 满载下载时 CPU 用户态占用 < 5%
- **并发**: 支持至少 50 个并发下载连接

### 平台兼容性
- Linux: 使用 `sendfile(2)` + `fadvise`
- Windows: 使用 `TransmitFile` (Win32 API) 或 fallback 到优化的异步分块传输
- macOS: 使用 `sendfile(2)`（FreeBSD 派生，兼容 Linux 接口）
