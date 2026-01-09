use russh_sftp::client::SftpSession;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SFTP file entry information
#[derive(Debug, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub permissions: u32,
}

/// SFTP client for file operations over SSH
pub struct SftpClient {
    session: Arc<Mutex<SftpSession>>,
}

impl std::fmt::Debug for SftpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpClient").finish()
    }
}

impl SftpClient {
    /// Create a new SFTP client from a russh_sftp session
    pub fn new(session: SftpSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// List directory contents
    pub async fn list_directory(&self, path: &str) -> anyhow::Result<Vec<SftpEntry>> {
        let session = self.session.lock().await;
        let dir = session.read_dir(path).await?;

        let mut entries = Vec::new();

        // Add parent directory entry
        if path != "/" {
            entries.push(SftpEntry {
                name: "..".to_string(),
                is_directory: true,
                size: 0,
                permissions: 0o755,
            });
        }

        for entry in dir {
            let filename = entry.file_name();

            // Skip hidden . and .. entries from the actual listing
            if filename == "." || filename == ".." {
                continue;
            }

            let is_dir = entry.file_type().is_dir();
            let size = entry.metadata().size.unwrap_or(0);
            let permissions = entry.metadata().permissions.unwrap_or(0);

            entries.push(SftpEntry {
                name: filename,
                is_directory: is_dir,
                size,
                permissions,
            });
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }

    /// Get the home directory
    pub async fn home_directory(&self) -> anyhow::Result<String> {
        let session = self.session.lock().await;
        let path = session.canonicalize(".").await?;
        Ok(path.to_string())
    }

    /// Download a file from the remote server
    pub async fn download_file(&self, remote_path: &str, local_path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        let data = session.read(remote_path).await?;
        tokio::fs::write(local_path, data).await?;
        Ok(())
    }

    /// Upload a file to the remote server
    pub async fn upload_file(&self, local_path: &str, remote_path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        let data = tokio::fs::read(local_path).await?;
        session.write(remote_path, &data).await?;
        Ok(())
    }

    /// Create a directory on the remote server
    pub async fn create_directory(&self, path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        session.create_dir(path).await?;
        Ok(())
    }

    /// Delete a file on the remote server
    pub async fn delete_file(&self, path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        session.remove_file(path).await?;
        Ok(())
    }

    /// Delete a directory on the remote server
    pub async fn delete_directory(&self, path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        session.remove_dir(path).await?;
        Ok(())
    }

    /// Rename/move a file or directory
    pub async fn rename(&self, old_path: &str, new_path: &str) -> anyhow::Result<()> {
        let session = self.session.lock().await;
        session.rename(old_path, new_path).await?;
        Ok(())
    }

    /// Get file/directory information
    pub async fn stat(&self, path: &str) -> anyhow::Result<SftpEntry> {
        let session = self.session.lock().await;
        let metadata = session.metadata(path).await?;

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(SftpEntry {
            name,
            is_directory: metadata.file_type().is_dir(),
            size: metadata.size.unwrap_or(0),
            permissions: metadata.permissions.unwrap_or(0),
        })
    }
}
