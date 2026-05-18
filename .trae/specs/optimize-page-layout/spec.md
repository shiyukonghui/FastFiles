# 页面布局优化 Spec

## Why
当前页面布局中，标题和说明文字占用空间，且「传输文件」按钮位于内容区域中央，不够醒目。将按钮移至顶部工具栏可以提升操作便捷性，同时移除冗余的标题说明使界面更简洁。

## What Changes
- 将「传输文件」按钮从内容区域移至顶部工具栏（AppBar）
- 移除页面内的标题（"FastFiles"）和说明文字（"局域网文件传输工具"）
- 调整页面布局，使共享文件列表直接展示在工具栏下方

## Impact
- Affected specs: 无
- Affected code: `src/App.tsx`

---

## ADDED Requirements

### Requirement: 顶部工具栏布局
系统 SHALL 在顶部工具栏（AppBar）中显示「传输文件」按钮，按钮位于工具栏右侧。

#### Scenario: 工具栏显示传输按钮
- **WHEN** 用户打开应用
- **THEN** 顶部工具栏右侧显示「传输文件」按钮，左侧显示应用名称

#### Scenario: 按钮功能保持不变
- **WHEN** 用户点击工具栏中的「传输文件」按钮
- **THEN** 系统打开文件选择对话框，功能与之前一致

---

## MODIFIED Requirements

### Requirement: 页面内容区域
系统 SHALL 在内容区域直接展示共享文件列表，不再显示标题和说明文字。

#### Scenario: 内容区域简洁展示
- **WHEN** 用户查看主页面
- **THEN** 内容区域仅显示错误提示（如有）和共享文件列表，无标题和说明

---

## REMOVED Requirements

### Requirement: 页面内标题和说明
**Reason**: 标题和说明文字占用空间，应用名称已显示在顶部工具栏，无需重复
**Migration**: 将标题移至工具栏左侧，移除说明文字
