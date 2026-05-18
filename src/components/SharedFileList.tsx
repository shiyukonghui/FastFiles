import { useState } from 'react';
import {
  List,
  ListItem,
  ListItemText,
  TextField,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogContentText,
  DialogActions,
  Button,
  Snackbar,
  Typography,
  Box,
  Stack,
  Alert,
  Chip,
} from '@mui/material';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DeleteIcon from '@mui/icons-material/Delete';
import RefreshIcon from '@mui/icons-material/Refresh';
import { formatFileSize } from '../utils/format';

export interface SharedFile {
  code: string;
  file_name: string;
  file_path: string;
  file_size: number;
}

export interface ServerInfo {
  ip: string;
  port: number;
  running: boolean;
  base_url: string;
  ips: string[];
  base_urls: string[];
}

interface SharedFileListProps {
  files: SharedFile[];
  serverInfo: ServerInfo;
  onDelete: (code: string) => void;
  onRefresh: () => void;
}

function SharedFileList({ files, serverInfo, onDelete, onRefresh }: SharedFileListProps) {
  const [snackbarOpen, setSnackbarOpen] = useState(false);
  const [snackbarMsg, setSnackbarMsg] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<SharedFile | null>(null);

  const handleCopy = async (text: string, msg: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setSnackbarMsg(msg);
      setSnackbarOpen(true);
    } catch {
      setSnackbarOpen(false);
    }
  };

  const handleConfirmDelete = () => {
    if (deleteTarget) {
      onDelete(deleteTarget.code);
      setDeleteTarget(null);
    }
  };

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

  return (
    <>
      <Box sx={{ mb: 3, p: 2, bgcolor: 'background.paper', borderRadius: 2 }}>
        <Typography variant="subtitle2" color="text.secondary" gutterBottom>
          分享链接
        </Typography>
        <Stack spacing={1}>
          {serverInfo.base_urls.map((baseUrl, idx) => (
            <Stack key={idx} direction="row" spacing={1} alignItems="center">
              <Chip
                label={serverInfo.ips[idx]}
                size="small"
                variant="outlined"
                sx={{ minWidth: 110, flexShrink: 0 }}
              />
              <TextField
                value={`${baseUrl}/fsf`}
                size="small"
                fullWidth
                variant="outlined"
                slotProps={{
                  input: {
                    readOnly: true,
                    sx: { fontSize: '0.85rem' },
                  },
                }}
                sx={{ flex: 1 }}
              />
              <IconButton
                aria-label="复制链接"
                onClick={() => handleCopy(`${baseUrl}/fsf`, '链接已复制到剪贴板')}
                size="small"
              >
                <ContentCopyIcon fontSize="small" />
              </IconButton>
            </Stack>
          ))}
        </Stack>
      </Box>

      <List>
        {files.map((file) => (
          <ListItem
            key={file.code}
            divider
            secondaryAction={
              <IconButton
                edge="end"
                aria-label="删除文件"
                onClick={() => setDeleteTarget(file)}
              >
                <DeleteIcon />
              </IconButton>
            }
            sx={{ pr: 6 }}
          >
            <ListItemText
              primary={file.file_name}
              primaryTypographyProps={{ fontWeight: 500, noWrap: true }}
              secondary={formatFileSize(file.file_size)}
              secondaryTypographyProps={{ component: 'span' }}
            />
            <Stack direction="row" spacing={1} alignItems="center" sx={{ ml: 2 }}>
              <Chip
                label={file.code}
                size="small"
                color="primary"
                variant="outlined"
                sx={{ fontWeight: 600, fontSize: '0.9rem' }}
              />
              <IconButton
                aria-label="复制验证码"
                onClick={() => handleCopy(file.code, '验证码已复制到剪贴板')}
                size="small"
              >
                <ContentCopyIcon fontSize="small" />
              </IconButton>
            </Stack>
          </ListItem>
        ))}
      </List>

      <Dialog open={deleteTarget !== null} onClose={() => setDeleteTarget(null)}>
        <DialogTitle>确认删除</DialogTitle>
        <DialogContent>
          <DialogContentText>
            确定要删除该共享文件吗？删除后验证码将立即失效。
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteTarget(null)}>取消</Button>
          <Button color="error" onClick={handleConfirmDelete}>
            删除
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar
        open={snackbarOpen}
        autoHideDuration={2000}
        onClose={() => setSnackbarOpen(false)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert severity="success" variant="filled" sx={{ width: '100%' }}>
          {snackbarMsg}
        </Alert>
      </Snackbar>
    </>
  );
}

export default SharedFileList;