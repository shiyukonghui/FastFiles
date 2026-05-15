use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest};
use actix_web::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use socket2::{Socket, Domain, Type, Protocol};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::file_manager::FileManager;

const DOWNLOAD_CHUNK_SIZE: usize = 32 * 1024 * 1024;
const CHANNEL_CAPACITY: usize = 16;

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn render_download_page(file_name: &str, file_size: u64, token: &str) -> String {
    let size_str = format_file_size(file_size);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>文件下载 - FastFiles</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: #f5f7fa;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            color: #333;
        }}
        .container {{
            background: #ffffff;
            border-radius: 16px;
            box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
            padding: 48px 40px;
            max-width: 480px;
            width: 90%;
            text-align: center;
        }}
        .icon {{
            width: 64px;
            height: 64px;
            margin: 0 auto 24px;
            background: #e8f0fe;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 32px;
        }}
        h2 {{
            font-size: 20px;
            font-weight: 600;
            margin-bottom: 8px;
            word-break: break-all;
            color: #1a1a1a;
        }}
        .file-meta {{
            font-size: 14px;
            color: #888;
            margin-bottom: 32px;
        }}
        .download-btn {{
            display: inline-block;
            background: #1a73e8;
            color: #ffffff;
            text-decoration: none;
            padding: 14px 48px;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 500;
            transition: background 0.2s ease;
            cursor: pointer;
            border: none;
        }}
        .download-btn:hover {{
            background: #1557b0;
        }}
        .footer {{
            margin-top: 32px;
            font-size: 12px;
            color: #bbb;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">📄</div>
        <h2>{file_name}</h2>
        <div class="file-meta">文件大小：{size_str}</div>
        <a class="download-btn" href="/api/download/{token}" download>下载文件</a>
        <div class="footer">由 FastFiles 提供</div>
    </div>
</body>
</html>"#,
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
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: #f5f7fa;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            color: #333;
        }
        .container {
            background: #ffffff;
            border-radius: 16px;
            box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
            padding: 48px 40px;
            max-width: 480px;
            width: 90%;
            text-align: center;
        }
        h1 {
            font-size: 72px;
            font-weight: 700;
            color: #1a73e8;
            margin-bottom: 16px;
        }
        h2 {
            font-size: 20px;
            font-weight: 600;
            margin-bottom: 12px;
            color: #1a1a1a;
        }
        p {
            font-size: 14px;
            color: #888;
            line-height: 1.6;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>404</h1>
        <h2>文件未找到</h2>
        <p>该下载链接可能已过期，或文件已被分享者删除。</p>
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

fn file_stream(
    path: std::path::PathBuf,
) -> impl futures_util::Stream<Item = Result<web::Bytes, actix_web::Error>> {
    let (tx, rx) = mpsc::channel::<Result<web::Bytes, std::io::Error>>(CHANNEL_CAPACITY);

    std::thread::spawn(move || {
        let mut file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                let _ = tx.blocking_send(Err(e));
                return;
            }
        };

        loop {
            let mut buf = vec![0u8; DOWNLOAD_CHUNK_SIZE];
            match std::io::Read::read(&mut file, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf.truncate(n);
                    if tx.blocking_send(Ok(web::Bytes::from(buf))).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });

    ReceiverStream::new(rx).map(|r| r.map_err(actix_web::Error::from))
}

async fn api_download(
    req: HttpRequest,
    fm: web::Data<std::sync::Mutex<FileManager>>,
) -> HttpResponse {
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

    let stream = file_stream(file_path);

    HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "application/octet-stream"))
        .insert_header((CONTENT_LENGTH, file_size))
        .insert_header((
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file_name),
        ))
        .streaming(stream)
}

fn create_listener() -> Result<(std::net::TcpListener, u16), std::io::Error> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.set_send_buffer_size(8 * 1024 * 1024)?;
    socket.set_recv_buffer_size(8 * 1024 * 1024)?;
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    let port = socket.local_addr()?
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

        let server = HttpServer::new(move || {
            App::new()
                .app_data(fm_data.clone())
                .route("/download/{token}", web::get().to(download_page))
                .route("/api/download/{token}", web::get().to(api_download))
        })
        .keep_alive(std::time::Duration::from_secs(60))
        .client_request_timeout(std::time::Duration::from_secs(0))
        .client_disconnect_timeout(std::time::Duration::from_secs(10))
        .backlog(1024)
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
