# 计划：移除首页空文件列表时的刷新按钮

## 背景
当前在 `SharedFileList.tsx` 组件中，当文件列表为空时会显示一个"刷新列表"按钮。但用户指出，没有文件时刷新功能并无实际价值，应该移除该按钮。

## 当前实现
在 `src/components/SharedFileList.tsx` 第 71-88 行，当 `files.length === 0` 时显示空状态提示，其中包含一个刷新按钮：

```tsx
if (files.length === 0) {
  return (
    <Box textAlign="center" sx={{ mt: 6 }}>
      <Typography variant="body1" color="text.secondary" gutterBottom>
        暂无共享文件，点击上方按钮选择文件开始分享
      </Typography>
      <Button
        variant="outlined"
        size="small"
        startIcon={<RefreshIcon />}
        onClick={onRefresh}
        sx={{ mt: 1 }}
      >
        刷新列表
      </Button>
    </Box>
  );
}
```

## 修改方案

### 步骤 1：移除刷新按钮
**文件**: `src/components/SharedFileList.tsx`

移除第 77-85 行的刷新按钮代码，只保留提示文字：

```tsx
if (files.length === 0) {
  return (
    <Box textAlign="center" sx={{ mt: 6 }}>
      <Typography variant="body1" color="text.secondary">
        暂无共享文件，点击上方按钮选择文件开始分享
      </Typography>
    </Box>
  );
}
```

### 步骤 2：清理未使用的导入
**文件**: `src/components/SharedFileList.tsx`

移除刷新按钮后，`RefreshIcon` 将不再被使用，需要从导入中移除：
- 移除第 23 行的 `import RefreshIcon from '@mui/icons-material/Refresh';`

### 步骤 3：清理未使用的 Props
**文件**: `src/components/SharedFileList.tsx` 和 `src/App.tsx`

由于 `onRefresh` 在组件中不再被使用，需要清理：
1. 从 `SharedFileListProps` 接口中移除 `onRefresh` 属性
2. 从 `App.tsx` 中移除传递给 `SharedFileList` 的 `onRefresh={refreshFiles}` 属性

> **重要说明**：此清理**不会影响**添加、删除文件后的自动刷新功能。
> - 添加文件后的刷新：在 `App.tsx` 的 `handleShare` 函数中直接调用 `refreshFiles()`
> - 删除文件后的刷新：在 `App.tsx` 的 `handleDelete` 函数中直接调用 `refreshFiles()`
> - `onRefresh` prop 仅用于空状态时的刷新按钮，移除它不会影响其他任何功能

## 影响范围
- 仅影响 `SharedFileList.tsx` 组件的空状态显示
- 不影响有文件时的正常功能
- 不影响其他组件

## 验证方式
1. 启动应用，确认空状态时不再显示刷新按钮
2. 添加文件后，确认文件列表正常显示
3. 删除所有文件后，确认空状态提示正常显示