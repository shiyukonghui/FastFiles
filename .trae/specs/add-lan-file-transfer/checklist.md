# Checklist

- [x] 项目初始化：Tauri v2 + React + TypeScript + MUI 项目结构正确，`cargo build` 和 `npm install` 均无报错
- [x] Rust 依赖：`Cargo.toml` 包含 tauri、actix-web、uuid、tokio 等必要依赖
- [x] 前端依赖：`package.json` 包含 @mui/material、@emotion/react、@emotion/styled、@mui/icons-material
- [x] 局域网 IP 检测：`detect_lan_ip()` 正确返回非回环 IPv4 地址，无网络时回退 127.0.0.1
- [x] 文件管理器：支持添加文件（生成 UUID token）、删除文件、查询文件、列出所有文件，线程安全
- [x] HTTP 服务器：随机端口启动成功，`/download/{token}` 返回下载页面，`/api/download/{token}` 流式下载
- [x] 下载页面：HTML 页面正确显示文件名和格式化后的文件大小（KB/MB/GB）
- [x] 流式下载：大文件下载不会导致内存飙升，Content-Disposition 设置为 attachment
- [x] 404 处理：访问不存在的 token 返回友好的 404 错误页面
- [x] 文件被删除处理：共享文件所在磁盘文件被删除后，访问链接返回 404
- [x] 端口冲突处理：随机端口被占用时自动重试，最多 10 次，全部失败返回错误
- [x] 前端「传输文件」按钮：点击后弹出系统文件选择对话框，支持多选
- [x] 共享文件列表：每项正确显示文件名、大小、下载链接、复制按钮、删除按钮
- [x] 复制链接：点击复制按钮后链接写入剪贴板，显示"已复制" Snackbar 提示
- [x] 删除确认：点击删除按钮弹出确认对话框，确认后移除文件，token 即时失效
- [x] 服务器生命周期：添加第一个文件启动服务器，删除最后一个文件关闭服务器
- [x] 数据无持久化：关闭应用再重启后，共享文件列表为空
- [x] 完整流程验收：选择文件 → 生成链接 → 局域网其他设备浏览器打开链接 → 显示下载页面 → 点击下载 → 文件成功下载
