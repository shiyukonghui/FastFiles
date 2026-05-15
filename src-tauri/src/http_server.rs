use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest, HttpResponseBuilder};
use actix_web::http::header::{
    CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE, CONTENT_RANGE,
    ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS,
};
use actix_web::body::BoxBody;
use bytes::Bytes;
use memmap2::Mmap;
use socket2::{Socket, Domain, Type, Protocol};
use std::io;
use std::sync::Arc;

use crate::file_manager::FileManager;
use crate::sendfile;

// mmap 包装器，用于 bytes::Bytes 的零拷贝视图
struct MmapBytes(Mmap);

impl AsRef<[u8]> for MmapBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// 安全：Mmap 是 Send + Sync（只读映射，不可变）
unsafe impl Send for MmapBytes {}
unsafe impl Sync for MmapBytes {}

/// 解析 Range 请求头
#[derive(Debug, Clone)]
struct HttpRange {
    start: u64,
    end: u64, // 包含边界
}

/// 多范围请求的各个段
#[derive(Debug, Clone)]
struct RangeRequest {
    ranges: Vec<HttpRange>,
}

fn parse_range_header(range_str: &str, file_size: u64) -> Option<RangeRequest> {
    let prefix = "bytes=";
    if !range_str.starts_with(prefix) {
        return None;
    }
    let ranges_part = &range_str[prefix.len()..];

    let mut ranges = Vec::new();
    for part in ranges_part.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }

        if part.starts_with('-') {
            // 后缀范围: bytes=-N（最后 N 字节）
            let suffix_len: u64 = part[1..].parse().ok()?;
            if suffix_len == 0 {
                return None;
            }
            let start = file_size.saturating_sub(suffix_len);
            let end = file_size.saturating_sub(1);
            ranges.push(HttpRange { start, end });
        } else if let Some(dash_pos) = part.find('-') {
            let start_str = &part[..dash_pos];
            let end_str = &part[dash_pos + 1..];

            let start: u64 = start_str.parse().ok()?;
            let end: u64 = if end_str.is_empty() {
                // 开放范围: bytes=N-（从 N 到文件末尾）
                file_size.saturating_sub(1)
            } else {
                end_str.parse().ok()?
            };

            if start > end || start >= file_size {
                return None; // 超出范围
            }
            ranges.push(HttpRange {
                start,
                end: end.min(file_size.saturating_sub(1)),
            });
        } else {
            return None;
        }
    }

    if ranges.is_empty() {
        None
    } else {
        Some(RangeRequest { ranges })
    }
}

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 根据文件扩展名返回对应的 Emoji 图标
fn file_icon(file_name: &str) -> &'static str {
    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "🗜️",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "🎬",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" => "🎵",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => "🖼️",
        "pdf" => "📕",
        "doc" | "docx" => "📝",
        "xls" | "xlsx" | "csv" => "📊",
        "ppt" | "pptx" => "📽️",
        "exe" | "msi" | "apk" | "dmg" => "⚙️",
        "iso" | "img" => "💿",
        "txt" | "md" | "log" | "json" | "xml" | "yaml" | "yml" => "📄",
        "psd" | "ai" => "🎨",
        "ttf" | "otf" | "woff" | "woff2" => "🔤",
        _ => "📦",
    }
}

fn render_download_page(file_name: &str, file_size: u64, token: &str) -> String {
    let size_str = format_file_size(file_size);
    let icon = file_icon(file_name);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>文件下载 - FastFiles</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Microsoft YaHei", sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            color: #333;
        }}
        .container {{
            background: #ffffff;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
            padding: 48px 44px;
            max-width: 500px;
            width: 92%;
            text-align: center;
        }}
        .file-icon {{
            font-size: 56px;
            margin-bottom: 20px;
            line-height: 1;
        }}
        h2 {{
            font-size: 22px;
            font-weight: 600;
            margin-bottom: 6px;
            word-break: break-all;
            color: #1a1a2e;
            line-height: 1.4;
        }}
        .file-meta {{
            font-size: 14px;
            color: #888;
            margin-bottom: 28px;
        }}
        .download-btn {{
            display: inline-block;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #ffffff;
            text-decoration: none;
            padding: 15px 52px;
            border-radius: 10px;
            font-size: 17px;
            font-weight: 600;
            transition: transform 0.2s ease, box-shadow 0.2s ease;
            cursor: pointer;
            border: none;
            letter-spacing: 0.5px;
        }}
        .download-btn:hover {{
            transform: translateY(-2px);
            box-shadow: 0 8px 25px rgba(102, 126, 234, 0.4);
        }}
        .features {{
            margin-top: 32px;
            padding: 20px 24px;
            background: #f8f9ff;
            border-radius: 12px;
            font-size: 13px;
            color: #666;
            text-align: left;
            line-height: 1.8;
        }}
        .features .title {{
            font-weight: 600;
            color: #444;
            margin-bottom: 6px;
            font-size: 14px;
        }}
        .features span.tag {{
            display: inline-block;
            background: #e8ecff;
            color: #5a67d8;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 500;
            margin-right: 4px;
        }}
        .cli-box {{
            margin-top: 16px;
            padding: 12px 16px;
            background: #1a1a2e;
            border-radius: 8px;
            color: #a0ffa0;
            font-family: "Cascadia Code", "Fira Code", "Consolas", monospace;
            font-size: 12px;
            text-align: left;
            overflow-x: auto;
            white-space: pre-wrap;
            word-break: break-all;
            line-height: 1.6;
        }}
        .footer {{
            margin-top: 28px;
            font-size: 12px;
            color: #ccc;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="file-icon">{icon}</div>
        <h2>{file_name}</h2>
        <div class="file-meta">文件大小：{size_str}</div>
        <a class="download-btn" href="/api/download/{token}" download>⬇ 下载文件</a>
        <div class="features">
            <div class="title">💡 下载提示</div>
            <span class="tag">断点续传</span> <span class="tag">多线程加速</span> <span class="tag">Range 支持</span>
            <div style="margin-top: 8px;">支持使用下载工具获得更快速度：</div>
        </div>
        <div class="cli-box"># aria2 多线程下载（16 线程）
aria2c -x 16 -s 16 "{{download_url}}"

# curl 断点续传
curl -C - -O "{{api_url}}"

# wget 继续下载
wget -c "{{api_url}}"</div>
        <div class="footer">由 <strong>FastFiles</strong> 提供 · 局域网极速传输</div>
    </div>
    <script>
        // 动态替换 CLI 命令中的 URL
        (function() {{
            var apiUrl = location.origin + '/api/download/{token}';
            var downloadUrl = location.origin + '/download/{token}';
            document.querySelectorAll('.cli-box').forEach(function(el) {{
                el.textContent = el.textContent.replace('{{{{download_url}}}}', downloadUrl).replace('{{{{api_url}}}}', apiUrl);
            }});
        }})();
    </script>
</body>
</html>"#,
        icon = icon,
        file_name = file_name,
        size_str = size_str,
        token = token,
    )
}

fn render_404_page() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - 文件未找到</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Microsoft YaHei", sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            color: #333;
        }
        .container {
            background: #ffffff;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
            padding: 48px 40px;
            max-width: 480px;
            width: 90%;
            text-align: center;
        }
        h1 {
            font-size: 84px;
            font-weight: 800;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 16px;
        }
        h2 {
            font-size: 20px;
            font-weight: 600;
            margin-bottom: 12px;
            color: #1a1a2e;
        }
        p {
            font-size: 14px;
            color: #888;
            line-height: 1.8;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>404</h1>
        <h2>文件未找到</h2>
        <p>该下载链接可能已过期，或文件已被分享者删除。<br>请联系分享者获取新的下载链接。</p>
    </div>
</body>
</html>"#.to_string()
}

async fn download_page(
    req: HttpRequest,
    fm: web::Data<std::sync::Mutex<FileManager>>,
) -> HttpResponse {
    let token = req.match_info().get("token").unwrap_or("");

    let shared_file = {
        let mgr = fm.lock().unwrap();
        mgr.get_file(token)
    };

    match shared_file {
        Some(file) => {
            let html = render_download_page(&file.file_name, file.file_size, &file.token);
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
        None => {
            let html = render_404_page();
            HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
    }
}

/// 用 mmap 打开文件并返回 Bytes（零拷贝视图）
fn mmap_file(path: &std::path::Path, file_size: u64) -> io::Result<Bytes> {
    if file_size == 0 {
        return Ok(Bytes::new());
    }

    let file = std::fs::File::open(path)?;

    // 通知操作系统预读优化（Linux）
    sendfile::fadvise_sequential(&file, file_size);

    // mmap 文件到虚拟地址空间（零拷贝，OS 按需分页）
    let mmap = unsafe { Mmap::map(&file)? };

    // 包装为 Bytes（引用计数管理 mmap 生命周期，无数据拷贝）
    let owner = MmapBytes(mmap);
    Ok(Bytes::from_owner(owner))
}

/// 构建 CORS 响应头
fn add_cors_headers(response: &mut HttpResponseBuilder) {
    response.insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
    response.insert_header((
        ACCESS_CONTROL_EXPOSE_HEADERS,
        "Content-Range, Accept-Ranges, Content-Length, Content-Disposition",
    ));
}

async fn api_download(
    req: HttpRequest,
    fm: web::Data<std::sync::Mutex<FileManager>>,
) -> HttpResponse<BoxBody> {
    let token = req.match_info().get("token").unwrap_or("");

    let shared_file = {
        let mgr = fm.lock().unwrap();
        mgr.get_file(token)
    };

    let file = match shared_file {
        Some(f) => f,
        None => {
            return HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(render_404_page());
        }
    };

    let file_path = std::path::PathBuf::from(&file.file_path);
    if !file_path.exists() {
        return HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(render_404_page());
    }

    let file_size = file.file_size;
    let file_name = file.file_name.clone();

    // mmap 文件为 Bytes（零拷贝）
    let full_bytes = match mmap_file(&file_path, file_size) {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("无法读取文件: {}", e));
        }
    };

    // 解析 Range 请求头
    let range_header = req
        .headers()
        .get(actix_web::http::header::RANGE)
        .and_then(|v| v.to_str().ok());

    match range_header.and_then(|r| parse_range_header(r, file_size)) {
        Some(range_req) if range_req.ranges.len() == 1 => {
            // 单范围请求 → 206 Partial Content
            let r = &range_req.ranges[0];
            let length = r.end - r.start + 1;
            let data = full_bytes.slice(r.start as usize..(r.end + 1) as usize);

            let mut response = HttpResponse::PartialContent();
            response
                .insert_header((CONTENT_TYPE, "application/octet-stream"))
                .insert_header((CONTENT_LENGTH, length))
                .insert_header((CONTENT_RANGE, format!("bytes {}-{}/{}", r.start, r.end, file_size)))
                .insert_header((ACCEPT_RANGES, "bytes"))
                .insert_header((
                    CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", file_name),
                ));
            add_cors_headers(&mut response);
            response.body(data)
        }
        Some(range_req) => {
            // 多范围请求 → 206 + multipart/byteranges
            let boundary = format!("fastfiles_boundary_{}", uuid::Uuid::new_v4());
            let content_type_val = format!("multipart/byteranges; boundary={}", boundary);
            let mut body_parts: Vec<u8> = Vec::new();

            for r in &range_req.ranges {
                let data = full_bytes.slice(r.start as usize..(r.end + 1) as usize);
                let part_header = format!(
                    "--{}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes {}-{}/{}\r\n\r\n",
                    boundary, r.start, r.end, file_size
                );
                body_parts.extend_from_slice(part_header.as_bytes());
                body_parts.extend_from_slice(&data);
                body_parts.extend_from_slice(b"\r\n");
            }
            body_parts.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

            let mut response = HttpResponse::PartialContent();
            response
                .insert_header((CONTENT_TYPE, content_type_val.as_str()))
                .insert_header((ACCEPT_RANGES, "bytes"));
            add_cors_headers(&mut response);
            response.body(body_parts)
        }
        None => {
            // 无 Range → 200 OK 完整文件
            let mut response = HttpResponse::Ok();
            response
                .insert_header((CONTENT_TYPE, "application/octet-stream"))
                .insert_header((CONTENT_LENGTH, file_size))
                .insert_header((ACCEPT_RANGES, "bytes"))
                .insert_header((
                    CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", file_name),
                ));
            add_cors_headers(&mut response);
            response.body(full_bytes)
        }
    }
}

/// 创建 TCP 监听器，配置最优传输参数
fn create_listener() -> io::Result<(std::net::TcpListener, u16)> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    // 端口复用
    socket.set_reuse_address(true)?;

    // TCP_NODELAY：禁用 Nagle 算法，消除小包延迟（对文件传输至关重要）
    socket.set_nodelay(true)?;

    // 最大化 socket 缓冲区（使用系统支持的最大值）
    let max_buf = sendfile::max_socket_buffer();
    let _ = socket.set_send_buffer_size(max_buf);
    let _ = socket.set_recv_buffer_size(max_buf);

    // TCP Keep-Alive 保持长连接存活
    let _ = socket.set_keepalive(true);
    #[cfg(target_os = "linux")]
    {
        // Linux TCP keepalive 参数：30s 空闲后开始探测，每 10s 探测一次
        let sock_ref = socket2::SockRef::from(&socket);
        let _ = sock_ref.set_tcp_keepalive(
            Some(std::time::Duration::from_secs(30)),
            Some(std::time::Duration::from_secs(10)),
            3,
        );
    }

    socket.bind(&addr.into())?;
    socket.listen(2048)?; // 增大 backlog 以支持高并发连接

    let port = socket
        .local_addr()?
        .as_socket_ipv4()
        .map(|a| a.port())
        .unwrap_or(0);
    let listener: std::net::TcpListener = socket.into();
    Ok((listener, port))
}

pub struct HttpFileServer {
    port: u16,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl HttpFileServer {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn start(
        file_manager: Arc<std::sync::Mutex<FileManager>>,
    ) -> Result<Self, String> {
        let (listener, port) = create_listener()
            .map_err(|e| format!("无法启动监听器: {}", e))?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let fm_data = web::Data::from(file_manager);

        // 使用 tokio 运行时 CPU 核心数作为 worker 数量
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(fm_data.clone())
                .route("/download/{token}", web::get().to(download_page))
                .route("/api/download/{token}", web::get().to(api_download))
        })
        .workers(worker_count)
        .keep_alive(std::time::Duration::from_secs(120))
        .client_request_timeout(std::time::Duration::from_secs(0)) // 无超时
        .client_disconnect_timeout(std::time::Duration::from_secs(30)) // 断开后快速清理
        .backlog(2048)
        .listen(listener)
        .map_err(|e| format!("绑定端口失败: {}", e))?
        .run();

        let handle = server.handle();

        tokio::spawn(server);

        tokio::spawn(async move {
            let _ = shutdown_rx.await;
            handle.stop(true).await;
        });

        Ok(HttpFileServer {
            port,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for HttpFileServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
