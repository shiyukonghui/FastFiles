use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest, HttpResponseBuilder};
use actix_web::http::header::{
    CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE, CONTENT_RANGE,
    ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS,
};
use actix_web::body::BoxBody;
use memmap2::Mmap;
use socket2::{Socket, Domain, Type, Protocol};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;

use crate::file_manager::FileManager;
use crate::sendfile;

const CHUNK_SIZE: usize = 64 * 1024 * 1024;
const CHANNEL_CAP: usize = 4;

struct MmapBytes(Mmap);

impl AsRef<[u8]> for MmapBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

unsafe impl Send for MmapBytes {}
unsafe impl Sync for MmapBytes {}

fn file_stream(
    path: std::path::PathBuf,
    start_offset: u64,
    length: u64,
) -> impl futures_util::Stream<Item = Result<web::Bytes, actix_web::Error>> {
    let (tx, rx) = mpsc::channel::<Result<web::Bytes, std::io::Error>>(CHANNEL_CAP);

    tokio::task::spawn_blocking(move || {
        if let Err(e) = stream_mmap(&path, start_offset, length, &tx) {
            stream_read(&path, start_offset, length, &tx, e);
        }
    });

    ReceiverStream::new(rx).map(|r| r.map_err(actix_web::Error::from))
}

fn stream_mmap(
    path: &std::path::Path,
    start_offset: u64,
    length: u64,
    tx: &mpsc::Sender<Result<web::Bytes, std::io::Error>>,
) -> io::Result<()> {
    let file = std::fs::File::open(path)?;

    if let Ok(meta) = file.metadata() {
        sendfile::fadvise_sequential(&file, meta.len());
    }

    let mmap = unsafe { Mmap::map(&file)? };
    let full_bytes = web::Bytes::from_owner(MmapBytes(mmap));

    let mut offset = start_offset as usize;
    let end = (start_offset + length) as usize;

    while offset < end {
        let chunk_end = (offset + CHUNK_SIZE).min(end);
        let chunk = full_bytes.slice(offset..chunk_end);

        if tx.blocking_send(Ok(chunk)).is_err() {
            break;
        }

        #[cfg(target_os = "linux")]
        unsafe {
            let ptr = full_bytes.as_ptr().add(offset) as *mut libc::c_void;
            let len = chunk_end - offset;
            libc::madvise(ptr, len, libc::MADV_DONTNEED);
        }

        offset = chunk_end;
    }

    Ok(())
}

fn stream_read(
    path: &std::path::Path,
    start_offset: u64,
    length: u64,
    tx: &mpsc::Sender<Result<web::Bytes, std::io::Error>>,
    _mmap_err: io::Error,
) {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.blocking_send(Err(e));
            return;
        }
    };

    if let Ok(meta) = file.metadata() {
        sendfile::fadvise_sequential(&file, meta.len());
    }

    if start_offset > 0 {
        use std::io::{Seek, SeekFrom};
        if let Err(e) = file.seek(SeekFrom::Start(start_offset)) {
            let _ = tx.blocking_send(Err(e));
            return;
        }
    }

    let mut remaining = length;
    let initial_cap = CHUNK_SIZE.min(remaining as usize).max(1);
    let mut buf = vec![0u8; initial_cap];

    while remaining > 0 {
        let to_read = buf.len().min(remaining as usize);
        match io::Read::read(&mut file, &mut buf[..to_read]) {
            Ok(0) => break,
            Ok(n) => {
                if tx.blocking_send(Ok(web::Bytes::copy_from_slice(&buf[..n]))).is_err() {
                    break;
                }
                remaining -= n as u64;
            }
            Err(e) => {
                let _ = tx.blocking_send(Err(e));
                break;
            }
        }
    }
}

fn read_file_range(path: &std::path::Path, start: u64, length: u64) -> io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    use std::io::{Read, Seek, SeekFrom};
    if start > 0 {
        file.seek(SeekFrom::Start(start))?;
    }
    let mut buf = vec![0u8; length as usize];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

#[derive(Debug, Clone)]
struct HttpRange {
    start: u64,
    end: u64,
}

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
                file_size.saturating_sub(1)
            } else {
                end_str.parse().ok()?
            };

            if start > end || start >= file_size {
                return None;
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

fn render_fsf_entry_page(error: Option<&str>) -> String {
    let error_html = if let Some(msg) = error {
        format!(
            r#"<div class="error-msg" style="color: #e74c3c; margin-bottom: 16px; font-size: 14px;">{}</div>"#,
            msg
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FastFiles 文件分享</title>
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
            max-width: 420px;
            width: 92%;
            text-align: center;
        }}
        .logo {{
            font-size: 48px;
            margin-bottom: 16px;
        }}
        h1 {{
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 8px;
            color: #1a1a2e;
        }}
        .subtitle {{
            font-size: 14px;
            color: #888;
            margin-bottom: 32px;
        }}
        .input-group {{
            display: flex;
            justify-content: center;
            gap: 8px;
            margin-bottom: 24px;
        }}
        .code-input {{
            width: 52px;
            height: 56px;
            text-align: center;
            font-size: 24px;
            font-weight: 600;
            text-transform: uppercase;
            border: 2px solid #e0e0e0;
            border-radius: 12px;
            outline: none;
            transition: border-color 0.2s, box-shadow 0.2s;
        }}
        .code-input:focus {{
            border-color: #667eea;
            box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.2);
        }}
        .submit-btn {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #ffffff;
            border: none;
            padding: 14px 48px;
            border-radius: 10px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s ease, box-shadow 0.2s ease;
        }}
        .submit-btn:hover {{
            transform: translateY(-2px);
            box-shadow: 0 8px 25px rgba(102, 126, 234, 0.4);
        }}
        .footer {{
            margin-top: 32px;
            font-size: 12px;
            color: #ccc;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">📁</div>
        <h1>FastFiles</h1>
        <div class="subtitle">请输入4位验证码获取文件</div>
        {error_html}
        <form action="/fsf/verify" method="GET" onsubmit="return handleSubmit(event)">
            <div class="input-group">
                <input type="text" class="code-input" maxlength="1" data-index="0" autocomplete="off">
                <input type="text" class="code-input" maxlength="1" data-index="1" autocomplete="off">
                <input type="text" class="code-input" maxlength="1" data-index="2" autocomplete="off">
                <input type="text" class="code-input" maxlength="1" data-index="3" autocomplete="off">
            </div>
            <input type="hidden" name="code" id="code">
            <button type="submit" class="submit-btn">获取文件</button>
        </form>
        <div class="footer">由 <strong>FastFiles</strong> 提供 · 局域网极速传输</div>
    </div>
    <script>
        const inputs = document.querySelectorAll('.code-input');
        inputs.forEach((input, idx) => {{
            input.addEventListener('input', (e) => {{
                const value = e.target.value.toUpperCase();
                e.target.value = value;
                if (value && idx < 3) {{
                    inputs[idx + 1].focus();
                }}
            }});
            input.addEventListener('keydown', (e) => {{
                if (e.key === 'Backspace' && !e.target.value && idx > 0) {{
                    inputs[idx - 1].focus();
                }}
            }});
            input.addEventListener('paste', (e) => {{
                e.preventDefault();
                const pasted = (e.clipboardData || window.clipboardData).getData('text').toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 4);
                pasted.split('').forEach((char, i) => {{
                    if (inputs[i]) inputs[i].value = char;
                }});
                if (pasted.length > 0) inputs[Math.min(pasted.length, 3)].focus();
            }});
        }});
        function handleSubmit(e) {{
            const code = Array.from(inputs).map(i => i.value.toUpperCase()).join('');
            if (code.length !== 4) {{
                e.preventDefault();
                alert('请输入4位验证码');
                return false;
            }}
            document.getElementById('code').value = code;
            return true;
        }}
        inputs[0].focus();
    </script>
</body>
</html>"#
    )
}

fn render_download_page(file_name: &str, file_size: u64, code: &str) -> String {
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
        <a class="download-btn" href="/api/fsf/download/{code}" download>⬇ 下载文件</a>
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
        (function() {{
            var apiUrl = location.origin + '/api/fsf/download/{code}';
            var downloadUrl = location.origin + '/fsf/verify?code={code}';
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
        code = code,
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
        .back-link {
            display: inline-block;
            margin-top: 24px;
            color: #667eea;
            text-decoration: none;
            font-weight: 500;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>404</h1>
        <h2>验证码无效</h2>
        <p>该验证码不存在或文件已被分享者删除。<br>请检查验证码是否正确，或联系分享者获取新的验证码。</p>
        <a class="back-link" href="/fsf">← 返回重新输入</a>
    </div>
</body>
</html>"#.to_string()
}

async fn fsf_entry_page() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_fsf_entry_page(None))
}

async fn fsf_verify_page(
    req: HttpRequest,
    fm: web::Data<std::sync::Mutex<FileManager>>,
) -> HttpResponse {
    let code = req
        .query_string()
        .split('&')
        .find_map(|pair| {
            let mut parts = pair.split('=');
            if parts.next() == Some("code") {
                parts.next()
            } else {
                None
            }
        })
        .unwrap_or("");

    let code = urlencoding_decode(code);

    if code.len() != 4 {
        return HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(render_fsf_entry_page(Some("请输入4位验证码")));
    }

    let shared_file = {
        let mgr = fm.lock().unwrap();
        mgr.get_file(&code)
    };

    match shared_file {
        Some(file) => {
            let html = render_download_page(&file.file_name, file.file_size, &file.code);
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
        None => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(render_fsf_entry_page(Some("验证码无效，请检查后重试")))
        }
    }
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let (Some(h), Some(l)) = (chars.next(), chars.next()) {
                if let (Some(h), Some(l)) = (h.to_digit(16), l.to_digit(16)) {
                    result.push(char::from_u32(h * 16 + l).unwrap_or(c));
                    continue;
                }
            }
        }
        result.push(c);
    }
    result
}

fn add_cors_headers(response: &mut HttpResponseBuilder) {
    response.insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
    response.insert_header((
        ACCESS_CONTROL_EXPOSE_HEADERS,
        "Content-Range, Accept-Ranges, Content-Length, Content-Disposition",
    ));
}

async fn api_fsf_download(
    req: HttpRequest,
    fm: web::Data<std::sync::Mutex<FileManager>>,
) -> HttpResponse<BoxBody> {
    let code = req.match_info().get("code").unwrap_or("");

    let shared_file = {
        let mgr = fm.lock().unwrap();
        mgr.get_file(code)
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

    let range_header = req
        .headers()
        .get(actix_web::http::header::RANGE)
        .and_then(|v| v.to_str().ok());

    match range_header.and_then(|r| parse_range_header(r, file_size)) {
        Some(range_req) if range_req.ranges.len() == 1 => {
            let r = &range_req.ranges[0];
            let length = r.end - r.start + 1;
            let stream = file_stream(file_path.clone(), r.start, length);

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
            response.streaming(stream)
        }
        Some(range_req) => {
            let boundary = format!("fastfiles_boundary_{}", uuid::Uuid::new_v4());
            let content_type_val = format!("multipart/byteranges; boundary={}", boundary);
            let mut body_parts: Vec<u8> = Vec::new();

            for r in &range_req.ranges {
                let length = r.end - r.start + 1;
                match read_file_range(&file_path, r.start, length) {
                    Ok(data) => {
                        let part_header = format!(
                            "--{}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes {}-{}/{}\r\n\r\n",
                            boundary, r.start, r.end, file_size
                        );
                        body_parts.extend_from_slice(part_header.as_bytes());
                        body_parts.extend_from_slice(&data);
                        body_parts.extend_from_slice(b"\r\n");
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError()
                            .body(format!("读取文件片段失败: {}", e));
                    }
                }
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
            let stream = file_stream(file_path, 0, file_size);

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
            response.streaming(stream)
        }
    }
}

fn create_listener() -> io::Result<(std::net::TcpListener, u16)> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    socket.set_reuse_address(true)?;
    socket.set_nodelay(true)?;

    let max_buf = sendfile::max_socket_buffer();
    let _ = socket.set_send_buffer_size(max_buf);
    let _ = socket.set_recv_buffer_size(max_buf);

    let _ = socket.set_keepalive(true);
    #[cfg(target_os = "linux")]
    {
        let sock_ref = socket2::SockRef::from(&socket);
        let keepalive = socket2::TcpKeepalive::new()
            .with_time(std::time::Duration::from_secs(30))
            .with_interval(std::time::Duration::from_secs(10));
        let _ = sock_ref.set_tcp_keepalive(&keepalive);
    }

    socket.bind(&addr.into())?;
    socket.listen(2048)?;

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

        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(fm_data.clone())
                .route("/fsf", web::get().to(fsf_entry_page))
                .route("/fsf/verify", web::get().to(fsf_verify_page))
                .route("/api/fsf/download/{code}", web::get().to(api_fsf_download))
        })
        .workers(worker_count)
        .keep_alive(std::time::Duration::from_secs(120))
        .client_request_timeout(std::time::Duration::from_secs(0))
        .client_disconnect_timeout(std::time::Duration::from_secs(30))
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
