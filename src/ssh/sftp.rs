use std::path::Path;

/// SFTP file entry information
#[derive(Debug, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub permissions: u32,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// SFTP client for file operations over SSH
#[derive(Debug)]
pub struct SftpClient {
    // russh_sftp session will be stored here
    // sftp: russh_sftp::client::SftpSession,
    current_path: String,
}

impl SftpClient {
    /// Create a new SFTP client from an SSH connection
    pub fn new() -> Self {
        Self {
            current_path: "/".to_string(),
        }
    }

    /// Get the current working directory
    pub fn current_path(&self) -> &str {
        &self.current_path
    }

    /// Change to a directory
    pub async fn change_directory(&mut self, path: &str) -> anyhow::Result<()> {
        // Resolve relative paths
        let new_path = if path.starts_with('/') {
            path.to_string()
        } else if path == ".." {
            Path::new(&self.current_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string())
        } else {
            format!("{}/{}", self.current_path.trim_end_matches('/'), path)
        };

        // TODO: Verify directory exists via SFTP
        self.current_path = new_path;
        Ok(())
    }

    /// List directory contents
    pub async fn list_directory(&self, path: Option<&str>) -> anyhow::Result<Vec<SftpEntry>> {
        let path = path.unwrap_or(&self.current_path);
        log::debug!("Listing directory: {}", path);

        // TODO: Implement actual SFTP directory listing
        // This is placeholder data for testing

        /*
        let entries = self.sftp.read_dir(path).await?;
        let mut result = Vec::new();

        for entry in entries {
            let name = entry.file_name();
            let attrs = entry.metadata();

            result.push(SftpEntry {
                name,
                is_directory: attrs.is_dir(),
                size: attrs.size().unwrap_or(0),
                permissions: attrs.permissions().unwrap_or(0),
                modified: attrs.modified().map(|t| chrono::DateTime::from(t)),
            });
        }
        */

        // Return placeholder entries for testing UI
        Ok(vec![
            SftpEntry {
                name: "..".to_string(),
                is_directory: true,
                size: 0,
                permissions: 0o755,
                modified: None,
            },
            SftpEntry {
                name: "Documents".to_string(),
                is_directory: true,
                size: 0,
                permissions: 0o755,
                modified: None,
            },
            SftpEntry {
                name: "Downloads".to_string(),
                is_directory: true,
                size: 0,
                permissions: 0o755,
                modified: None,
            },
            SftpEntry {
                name: ".bashrc".to_string(),
                is_directory: false,
                size: 3771,
                permissions: 0o644,
                modified: None,
            },
            SftpEntry {
                name: "notes.txt".to_string(),
                is_directory: false,
                size: 1234,
                permissions: 0o644,
                modified: None,
            },
        ])
    }

    /// Download a file from the remote server
    pub async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        _progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> anyhow::Result<()> {
        log::info!("Downloading {} to {}", remote_path, local_path);

        // TODO: Implement actual file download
        /*
        let mut remote_file = self.sftp.open(remote_path).await?;
        let mut local_file = tokio::fs::File::create(local_path).await?;

        let mut total_read = 0u64;
        let file_size = remote_file.metadata().await?.size().unwrap_or(0);

        let mut buffer = vec![0u8; 32768];
        loop {
            let n = remote_file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            local_file.write_all(&buffer[..n]).await?;
            total_read += n as u64;

            if let Some(ref callback) = progress_callback {
                callback(total_read, file_size);
            }
        }
        */

        Ok(())
    }

    /// Upload a file to the remote server
    pub async fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
        _progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> anyhow::Result<()> {
        log::info!("Uploading {} to {}", local_path, remote_path);

        // TODO: Implement actual file upload
        /*
        let mut local_file = tokio::fs::File::open(local_path).await?;
        let file_size = local_file.metadata().await?.len();

        let mut remote_file = self.sftp.create(remote_path).await?;

        let mut total_written = 0u64;
        let mut buffer = vec![0u8; 32768];

        loop {
            let n = local_file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            remote_file.write_all(&buffer[..n]).await?;
            total_written += n as u64;

            if let Some(ref callback) = progress_callback {
                callback(total_written, file_size);
            }
        }
        */

        Ok(())
    }

    /// Create a directory on the remote server
    pub async fn create_directory(&self, path: &str) -> anyhow::Result<()> {
        log::info!("Creating directory: {}", path);
        // TODO: self.sftp.create_dir(path).await?;
        Ok(())
    }

    /// Delete a file on the remote server
    pub async fn delete_file(&self, path: &str) -> anyhow::Result<()> {
        log::info!("Deleting file: {}", path);
        // TODO: self.sftp.remove_file(path).await?;
        Ok(())
    }

    /// Delete a directory on the remote server
    pub async fn delete_directory(&self, path: &str) -> anyhow::Result<()> {
        log::info!("Deleting directory: {}", path);
        // TODO: self.sftp.remove_dir(path).await?;
        Ok(())
    }

    /// Rename/move a file or directory
    pub async fn rename(&self, old_path: &str, new_path: &str) -> anyhow::Result<()> {
        log::info!("Renaming {} to {}", old_path, new_path);
        // TODO: self.sftp.rename(old_path, new_path).await?;
        Ok(())
    }

    /// Get file information
    pub async fn stat(&self, path: &str) -> anyhow::Result<SftpEntry> {
        log::debug!("Getting stats for: {}", path);

        // TODO: Implement actual stat
        /*
        let attrs = self.sftp.metadata(path).await?;
        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(SftpEntry {
            name,
            is_directory: attrs.is_dir(),
            size: attrs.size().unwrap_or(0),
            permissions: attrs.permissions().unwrap_or(0),
            modified: attrs.modified().map(|t| chrono::DateTime::from(t)),
        })
        */

        Ok(SftpEntry {
            name: Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            is_directory: false,
            size: 0,
            permissions: 0o644,
            modified: None,
        })
    }
}

impl Default for SftpClient {
    fn default() -> Self {
        Self::new()
    }
}
