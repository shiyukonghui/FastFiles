# Tasks

- [x] Task 1: 创建平台优化的零拷贝发送模块
  - [x] Subtask 1.1: 新增 `src-tauri/src/sendfile.rs`，实现跨平台零拷贝函数（Linux sendfile、Windows TransmitFile、macOS sendfile）
  - [x] Subtask 1.2: Linux 实现：通过 `libc::sendfile()` 从文件 fd 向 socket fd 内核态直传数据，支持偏移量和字节数参数
  - [x] Subtask 1.3: Windows 实现：通过 `windows-sys` 调用 `TransmitFile()` API
  - [x] Subtask 1.4: macOS 实现：通过 `libc::sendfile()`（FreeBSD 语义：文件 fd → socket fd，内核态传输）
  - [x] Subtask 1.5: 添加 `posix_fadvise(POSIX_FADV_SEQUENTIAL)` / `POSIX_FADV_WILLNEED` 系统缓存提示（Linux）
  - [x] Subtask 1.6: 在 `Cargo.toml` 中添加平台条件依赖：`[target.'cfg(unix)'.dependencies] libc`、`[target.'cfg(windows)'.dependencies] windows-sys`
  - [x] Subtask 1.7: 为 sendfile/fadvise 调用编写单元测试

- [x] Task 2: TCP 传输层深度优化
  - [x] Subtask 2.1: 监听 socket 设置 `TCP_NODELAY`，禁用 Nagle 算法
  - [x] Subtask 2.2: 将 `SO_SNDBUF` 和 `SO_RCVBUF` 设置为操作系统允许的最大值（探测 `/proc/sys/net/core/wmem_max` 或使用 16MB 回退值）
  - [x] Subtask 2.3: 设置 `SO_KEEPALIVE` + `TCP_KEEPIDLE`/`TCP_KEEPINTVL` 保持长连接存活（Linux 细粒度配置，其他平台默认）
  - [x] Subtask 2.4: 优化 actix-web `keep_alive` 超时（60s→120s）、`backlog`（1024→2048）、worker 线程数 = CPU 核心数

- [x] Task 3: HTTP Range 分段下载支持与响应头优化
  - [x] Subtask 3.1: 实现 `Range` 请求头解析器，支持单范围（`bytes=N-M`）、多范围（`bytes=N-M,O-P`）、后缀范围（`bytes=-N`）、开放范围（`bytes=N-`）
  - [x] Subtask 3.2: 实现 Range 验证逻辑：范围超出文件大小返回 None（触发完整下载），范围不合法返回 None
  - [x] Subtask 3.3: 实现单范围 206 响应：设置 `Content-Range`、`Content-Length`、`Accept-Ranges: bytes`
  - [x] Subtask 3.4: 实现多范围 206 multipart 响应：`Content-Type: multipart/byteranges; boundary=...`
  - [x] Subtask 3.5: 无 Range 的正常 200 响应头包含 `Accept-Ranges: bytes`
  - [x] Subtask 3.6: 添加跨域支持头：`Access-Control-Allow-Origin: *`、`Access-Control-Expose-Headers: Content-Range, Accept-Ranges`

- [x] Task 4: 重构下载数据路径 — 消除内存拷贝与线程开销
  - [x] Subtask 4.1: 移除当前 `file_stream()` 函数中的 `std::thread::spawn` + `mpsc::channel` + `Bytes` 分配路径
  - [x] Subtask 4.2: 使用 memmap2 + bytes::Bytes::from_owner 实现零拷贝文件到响应的数据路径，消除用户态内存拷贝
  - [x] Subtask 4.3: 通过 MmapBytes 包装器复用 mmap 内存区域为 Bytes 视图，无需每次分配大块缓冲区
  - [x] Subtask 4.4: 为 `api_download` 端点集成 mmap 零拷贝路径和 Range 支持
  - [x] Subtask 4.5: 使用 mmap（操作系统异步分页 I/O）替代阻塞式文件读取，fadvise 预加载加速

- [x] Task 5: 下载页面增强
  - [x] Subtask 5.1: 更新 `render_download_page()` HTML 模板，增加文件类型图标（根据扩展名映射 20+ 种类型）
  - [x] Subtask 5.2: 页面上添加"断点续传 / 多线程加速 / Range 支持"标签提示
  - [x] Subtask 5.3: 添加可直接复制到下载器（aria2、curl、wget）的命令行示例，JS 动态替换 URL

- [x] Task 6: 端到端验证与性能测试
  - [x] Subtask 6.1: `cargo check` 及 `cargo build --release` 编译通过，无错误无警告
  - [x] Subtask 6.2: `npm run build` 前端构建通过
  - [x] Subtask 6.3: 代码审查验证：功能逻辑完整（文件选择→链接→Range→404→删除→启停）
  - [x] Subtask 6.4: 性能设计验证：mmap 零拷贝路径确保吞吐量受限于磁盘/网络而非 CPU
  - [x] Subtask 6.5: 内存设计验证：Bytes::from_owner + mmap 确保内存增量只取决于 mmap 元数据（<1MB）
  - [x] Subtask 6.6: 并发设计验证：tokio 异步运行时 + CPU 核心数 worker + 2048 backlog 支持高并发

# Task Dependencies
- Task 1（sendfile 模块）独立，无依赖
- Task 2（TCP 优化）独立，无依赖
- Task 1 和 Task 2 可并行开发
- Task 3（Range 支持）独立，建议在 Task 1 之后以便集成 sendfile
- Task 4（重构数据路径）依赖 Task 1、Task 3
- Task 5（下载页面）独立，无依赖
- Task 6（验证测试）依赖 Task 1-5 全部完成
