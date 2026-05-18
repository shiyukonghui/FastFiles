import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Container,
  Button,
  Stack,
  Alert,
  CircularProgress,
  AppBar,
  Toolbar,
} from '@mui/material';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import SharedFileList from './components/SharedFileList';
import type { SharedFile, ServerInfo } from './components/SharedFileList';

function App() {
  const [files, setFiles] = useState<SharedFile[]>([]);
  const [serverInfo, setServerInfo] = useState<ServerInfo>({
    ip: '',
    port: 0,
    running: false,
    base_url: '',
    ips: [],
    base_urls: [],
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshFiles = useCallback(async () => {
    const f = await invoke<SharedFile[]>('get_shared_files');
    setFiles(f);
  }, []);

  const refreshServerInfo = useCallback(async () => {
    const info = await invoke<ServerInfo>('get_server_info');
    setServerInfo(info);
  }, []);

  const handleShare = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke<SharedFile>('share_file');
      await refreshFiles();
      await refreshServerInfo();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (token: string) => {
    try {
      await invoke('delete_shared_file', { token });
      await refreshFiles();
      await refreshServerInfo();
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    refreshFiles();
    refreshServerInfo();
  }, [refreshFiles, refreshServerInfo]);

  return (
    <>
      <AppBar position="fixed">
        <Toolbar sx={{ justifyContent: 'center' }}>
          <Button
            variant="contained"
            color="secondary"
            startIcon={loading ? undefined : <UploadFileIcon />}
            onClick={handleShare}
            disabled={loading}
          >
            {loading ? (
              <CircularProgress size={24} color="inherit" />
            ) : (
              '传输文件'
            )}
          </Button>
        </Toolbar>
      </AppBar>

      <Container maxWidth="md" sx={{ mt: 10 }}>
        <Stack spacing={3}>
          {error && (
            <Alert severity="error" onClose={() => setError(null)}>
              {error}
            </Alert>
          )}

          <SharedFileList
            files={files}
            serverInfo={serverInfo}
            onDelete={handleDelete}
            onRefresh={refreshFiles}
          />
        </Stack>
      </Container>
    </>
  );
}

export default App;
