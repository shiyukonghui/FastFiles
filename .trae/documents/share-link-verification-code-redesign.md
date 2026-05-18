# 分享链接验证码重设计方案

## 需求分析

### 当前状态
- 分享链接格式：`http://{ip}:{port}/download/{token}`
- Token 使用 UUID v4（36字符），安全但难以记忆和使用
- 每个文件对应一个完整的 URL
- 无验证码机制

### 目标状态
- 入口 URL：`http://{ip}:{port}/fsf`（统一入口）
- 打开 `/fsf` 显示验证码输入页面（类似网盘）
- 前端文件列表：文件列表顶部展示统一 URL，每个文件展示4位验证码
- 用户输入验证码后才能下载对应文件

## 架构设计

### URL 结构变更
```
旧：http://{ip}:{port}/download/{36位UUID}
新：http://{ip}:{port}/fsf （入口页面）
    http://{ip}:{port}/fsf/verify/{4位验证码} （验证并下载）
```

### 数据结构变更

**SharedFile 结构体**（file_manager.rs）：
```rust
pub struct SharedFile {
    pub code: String,        // 4位验证码（替代 token）
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
}
```

### 页面流程
```
用户访问 /fsf
    ↓
显示验证码输入页面
    ↓
用户输入4位验证码
    ↓
POST /fsf/verify → 验证验证码
    ↓
验证成功 → 显示文件下载页面
验证失败 → 显示错误提示
```

## 实现步骤

### 第一阶段：后端改造

#### 1.1 修改 FileManager（file_manager.rs）
- [ ] 将 `token` 字段改为 `code`
- [ ] 修改 `add_file()` 方法：生成4位随机验证码（数字+字母）
- [ ] 添加 `verify_code()` 方法：验证验证码是否存在
- [ ] 更新 `get_file()` 方法：使用 code 查询

#### 1.2 新增路由和页面（http_server.rs）
- [ ] 添加 `/fsf` 路由：显示验证码输入页面
- [ ] 添加 `/fsf/verify/{code}` 路由：验证并显示下载页面
- [ ] 添加 `/api/fsf/download/{code}` 路由：实际文件下载 API
- [ ] 移除旧的 `/download/{token}` 和 `/api/download/{token}` 路由

#### 1.3 新增 HTML 页面
- [ ] `render_fsf_entry_page()`：验证码输入页面（类似网盘风格）
- [ ] 修改 `render_download_page()`：适配新的 URL 结构

### 第二阶段：前端改造

#### 2.1 修改 SharedFileList 组件（SharedFileList.tsx）
- [ ] 顶部展示统一 URL：`http://{ip}:{port}/fsf`
- [ ] 每个文件项展示4位验证码（可复制）
- [ ] 移除每个文件的完整 URL 展示

#### 2.2 更新类型定义
- [ ] 修改 `SharedFile` 接口：`token` → `code`
- [ ] 更新 `ServerInfo` 接口（如需要）

### 第三阶段：命令和 API 更新

#### 3.1 修改 Tauri 命令（commands.rs）
- [ ] 更新 `share_file()` 返回值：返回验证码而非完整 URL
- [ ] 更新 `get_shared_files()` 返回值
- [ ] 更新 `remove_shared_file()` 参数

## 详细设计

### 验证码生成算法
```rust
fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
```
- 排除易混淆字符：0/O, 1/I/L
- 4位验证码，约 32^4 = 1,048,576 种组合

### 验证码输入页面设计
- 简洁的输入框（4个格子或单个输入框）
- 提交按钮
- 错误提示区域
- 品牌标识

### 前端展示布局
```
┌─────────────────────────────────────────────┐
│  分享链接: http://10.19.129.102:6165/fsf    │
│  [复制链接]                                  │
├─────────────────────────────────────────────┤
│  文件名                    大小    验证码    │
│  report.pdf               2.5MB   A3K7 [复制]│
│  video.mp4               1.2GB   B8M2 [复制]│
│  data.zip                500MB   C5N9 [复制]│
└─────────────────────────────────────────────┘
```

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src-tauri/src/file_manager.rs` | 验证码生成、数据结构 |
| `src-tauri/src/http_server.rs` | 新路由、新页面渲染 |
| `src-tauri/src/commands.rs` | Tauri 命令更新 |
| `src/components/SharedFileList.tsx` | 前端展示改造 |

## 兼容性考虑
- 直接移除，因为这是内部工具，用户可以快速升级

## 安全性分析

- 4位验证码（32字符集）提供约 100 万种组合
- 对于局域网场景，安全性足够
- 相比 UUID v4（2^122 种组合），安全性降低，但更易用
- 可考虑添加验证失败次数限制（防止暴力破解）
