use std::io;

/// 平台优化：通知操作系统即将顺序读取文件，预加载到页缓存
/// Linux: posix_fadvise(POSIX_FADV_SEQUENTIAL | POSIX_FADV_WILLNEED)
/// Windows/macOS: 无操作（系统默认行为近似）
pub fn fadvise_sequential(_file: &std::fs::File, file_size: u64) {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let fd = _file.as_raw_fd();
        // POSIX_FADV_SEQUENTIAL = 2, POSIX_FADV_WILLNEED = 3
        // 建议内核：顺序访问模式 + 预读整个文件
        unsafe {
            libc::posix_fadvise64(fd, 0, file_size as libc::off64_t, libc::POSIX_FADV_SEQUENTIAL);
            libc::posix_fadvise64(fd, 0, file_size as libc::off64_t, libc::POSIX_FADV_WILLNEED);
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (_file, file_size);
}

/// 获取 sockopt 支持的最大 socket 缓冲区大小
/// 在 Linux 上从 /proc/sys/net/core/wmem_max 读取
pub fn max_socket_buffer() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(val) = std::fs::read_to_string("/proc/sys/net/core/wmem_max") {
            if let Ok(max) = val.trim().parse::<usize>() {
                return max;
            }
        }
    }
    16 * 1024 * 1024 // 回退值：16MB
}

/// 内核级零拷贝发送：从文件 fd 向 socket fd 直接传输
/// Linux: sendfile64()
/// macOS: sendfile() with FreeBSD semantics (header+trailer not used)
/// 注意：需要在 actix-web 外部通过 raw socket 使用
#[cfg(unix)]
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn raw_sendfile(
    out_fd: std::os::fd::RawFd,
    in_fd: std::os::fd::RawFd,
    offset: &mut u64,
    count: usize,
) -> io::Result<usize> {
    #[cfg(target_os = "linux")]
    {
        let result = unsafe {
            libc::sendfile64(
                out_fd,
                in_fd,
                offset as *mut u64 as *mut libc::off64_t,
                count,
            )
        };
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result as usize)
        }
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        let mut sent: libc::off_t = 0;
        // macOS sendfile: sendfile(fd, socket, offset, &len, NULL, 0)
        let result = unsafe {
            libc::sendfile(
                in_fd,
                out_fd,
                *offset as libc::off_t,
                &mut sent,
                std::ptr::null_mut(),
                0,
            )
        };
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            *offset += sent as u64;
            Ok(sent as usize)
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    {
        let _ = (out_fd, in_fd, offset, count);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "平台不支持 sendfile",
        ))
    }
}

/// Windows TransmitFile 零拷贝发送
/// 从文件句柄向 socket 直接传输
#[cfg(windows)]
pub fn transmit_file(
    socket: std::os::windows::raw::SOCKET,
    file_handle: std::os::windows::raw::HANDLE,
    _offset: u64,
    bytes_to_write: u32,
) -> io::Result<()> {
    use windows_sys::Win32::Networking::WinSock::TransmitFile;
    use windows_sys::Win32::Networking::WinSock::TF_WRITE_BEHIND;

    let mut overlapped = unsafe { std::mem::zeroed() };
    let result = unsafe {
        TransmitFile(
            socket as _,
            file_handle as _,
            bytes_to_write,
            0,
            &mut overlapped,
            std::ptr::null_mut(),
            TF_WRITE_BEHIND,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_fadvise_does_not_crash() {
        let mut tmp = tempfile::tempfile().unwrap();
        tmp.write_all(b"hello world").unwrap();
        fadvise_sequential(&tmp, 11);
    }

    #[test]
    fn test_max_socket_buffer_returns_reasonable_value() {
        let buf = max_socket_buffer();
        assert!(buf >= 8 * 1024 * 1024, "socket buffer should be at least 8MB");
    }
}
