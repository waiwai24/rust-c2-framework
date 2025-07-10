use bytes::Bytes;
use common::error::C2Result;
use common::message::{FileChunk, FileEntry};
use futures::stream::{self, Stream};
use futures::StreamExt as FuturesStreamExt; // Import StreamExt for .boxed() with alias
use log::{error, info};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::{io, path::Path};
use tokio::sync::Mutex;
use tokio::{
    fs as tokio_fs,
    fs::File as TokioFile,
    io::{AsyncReadExt, AsyncWriteExt},
};
use uuid::Uuid; // Required for Pin

const CHUNK_SIZE: usize = 8192; // 8KB chunks for file transfer

pub struct ClientFileManager {
    // Store ongoing download streams, keyed by a unique file_id
    // Arc<Mutex<...>> is used for thread-safe access to the stream
    ongoing_downloads: Arc<
        Mutex<
            HashMap<
                String,
                Pin<Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + Sync + 'static>>,
            >,
        >,
    >,
    // Store ongoing upload files, keyed by a unique file_id
    ongoing_uploads: Arc<Mutex<HashMap<String, TokioFile>>>,
}

impl ClientFileManager {
    pub fn new() -> Self {
        Self {
            ongoing_downloads: Arc::new(Mutex::new(HashMap::new())),
            ongoing_uploads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Lists files and directories in a given path, with optional recursion.
    pub async fn list_directory(path: &Path, recursive: bool) -> C2Result<Vec<FileEntry>> {
        info!(
            "Attempting to list directory on client: {:?}, recursive: {}",
            path, recursive
        );
        let mut entries = Vec::new();
        let mut stack = vec![path.to_path_buf()];

        while let Some(current_path) = stack.pop() {
            info!("Reading directory: {:?}", current_path);

            let mut read_dir = match tokio_fs::read_dir(&current_path).await {
                Ok(dir) => dir,
                Err(e) => {
                    error!("Failed to read directory {:?}: {}", current_path, e);
                    return Err(e.into());
                }
            };

            while let Some(entry) = read_dir.next_entry().await? {
                let entry_path = entry.path();
                let metadata = match tokio_fs::metadata(&entry_path).await {
                    Ok(meta) => meta,
                    Err(e) => {
                        error!("Failed to get metadata for {:?}: {}", entry_path, e);
                        continue;
                    }
                };

                let file_type = metadata.file_type();
                let is_dir = file_type.is_dir();

                let permissions = format!("{:?}", metadata.permissions());

                let file_entry = FileEntry {
                    name: entry_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    path: entry_path.clone(),
                    is_dir,
                    size: if file_type.is_file() {
                        Some(metadata.len())
                    } else {
                        None
                    },
                    modified: metadata.modified().ok(),
                    permissions: Some(permissions),
                    owner: None, // Set to None as ownership info is not easily available on all platforms
                    group: None, // Set to None as group info is not easily available on all platforms
                };

                info!("Found entry: {:?}", file_entry);
                entries.push(file_entry);

                if recursive && is_dir {
                    stack.push(entry_path);
                }
            }
        }

        info!(
            "Successfully listed directory on client: {:?}, found {} entries",
            path,
            entries.len()
        );
        Ok(entries)
    }

    /// Deletes a file or directory on the client.
    pub async fn delete_path(path: &Path) -> C2Result<()> {
        info!("Attempting to delete path on client: {:?}", path);

        if !path.exists() {
            error!("Delete failed on client: Path {:?} not found.", path);
            return Err(io::Error::new(io::ErrorKind::NotFound, "Path not found").into());
        }

        let metadata = tokio_fs::metadata(path).await?;
        if metadata.is_dir() {
            tokio_fs::remove_dir_all(path).await?;
            info!("Successfully deleted directory on client: {:?}", path);
        } else {
            tokio_fs::remove_file(path).await?;
            info!("Successfully deleted file on client: {:?}", path);
        }
        Ok(())
    }

    /// Initiates reading a file in chunks for download from client to server.
    pub async fn initiate_download(&self, path: &Path) -> C2Result<String> {
        info!("Attempting to initiate file download on client: {:?}", path);

        let file = TokioFile::open(path).await?;
        let file_id = Uuid::new_v4().to_string();

        // Clone path to be owned by the async block, ensuring 'static lifetime for the stream
        let owned_path = path.to_path_buf();

        let stream: Pin<Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + Sync + 'static>> =
            Box::pin(futures::StreamExt::map(
                // Explicitly use futures::StreamExt::map
                stream::unfold(file, move |mut file| {
                    let path_clone = owned_path.clone();
                    async move {
                        let mut buffer = vec![0; CHUNK_SIZE];
                        match file.read(&mut buffer).await {
                            Ok(0) => None,
                            Ok(n) => Some((Ok(Bytes::copy_from_slice(&buffer[..n])), file)),
                            Err(e) => {
                                error!("Error reading file chunk from {:?}: {}", path_clone, e);
                                Some((Err(e), file))
                            }
                        }
                    }
                }),
                |res| res.map_err(|e| e.into()), // Convert io::Error to C2Error
            ));

        self.ongoing_downloads
            .lock()
            .await
            .insert(file_id.clone(), stream);
        info!(
            "Successfully initiated file download for: {:?} with ID: {}",
            path, file_id
        );
        Ok(file_id)
    }

    /// Gets the next chunk for an ongoing download.
    pub async fn get_next_download_chunk(&self, file_id: &str) -> C2Result<Option<FileChunk>> {
        let mut downloads = self.ongoing_downloads.lock().await;
        if let Some(stream) = downloads.get_mut(file_id) {
            match FuturesStreamExt::next(stream).await {
                // Explicitly use futures::StreamExt::next
                Some(Ok(bytes)) => {
                    // is_last will be determined by the server when it receives None
                    Ok(Some(FileChunk {
                        file_id: file_id.to_string(),
                        chunk: bytes.to_vec(),
                        is_last: false, // This will be set to true when None is returned
                        offset: 0, // Offset will be managed by the server/client during transfer
                    }))
                }
                Some(Err(e)) => {
                    error!("Error getting next download chunk for {}: {}", file_id, e);
                    downloads.remove(file_id); // Remove stream on error
                    Err(e.into())
                }
                None => {
                    info!("Download stream for {} completed.", file_id);
                    downloads.remove(file_id); // Remove stream when done
                    Ok(None)
                }
            }
        } else {
            error!("No ongoing download found for file_id: {}", file_id);
            Err(io::Error::new(io::ErrorKind::NotFound, "Download not found").into())
        }
    }

    /// Initiates writing a file for upload to the client.
    pub async fn initiate_upload(&self, path: &Path, file_id: &str) -> C2Result<()> {
        info!(
            "Attempting to initiate file upload on client: {:?} with ID: {}",
            path, file_id
        );

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }

        let file = TokioFile::create(path).await?; // Create or truncate the file
        self.ongoing_uploads
            .lock()
            .await
            .insert(file_id.to_string(), file);
        info!(
            "Successfully initiated file upload for: {:?} with ID: {}",
            path, file_id
        );
        Ok(())
    }

    /// Writes chunks to a file for upload to the client.
    pub async fn write_file_chunk(&self, file_id: &str, chunk: FileChunk) -> C2Result<()> {
        info!(
            "Attempting to write file chunk for ID: {} (offset: {})",
            file_id, chunk.offset
        );
        let mut uploads = self.ongoing_uploads.lock().await;
        if let Some(file) = uploads.get_mut(file_id) {
            // Seek to the correct offset before writing
            tokio::io::AsyncSeekExt::seek(file, io::SeekFrom::Start(chunk.offset)).await?;
            file.write_all(&chunk.chunk).await?;
            info!(
                "Successfully wrote chunk for file ID: {} ({} bytes)",
                file_id,
                chunk.chunk.len()
            );

            if chunk.is_last {
                info!("Upload stream for {} completed.", file_id);
                uploads.remove(file_id); // Remove stream when done
            }
            Ok(())
        } else {
            error!("No ongoing upload found for file_id: {}", file_id);
            Err(io::Error::new(io::ErrorKind::NotFound, "Upload not found").into())
        }
    }
}
