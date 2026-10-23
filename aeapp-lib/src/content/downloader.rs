use std::{
    cmp,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    pin::Pin,
    prelude,
    sync::{Arc, Mutex},
    task,
};

use crate::{
    content::{
        manager::DownloaderError,
        zip::{CompressionType, ZipFile, ZipFileEntry},
        zlib::{restore_zlib_state, write_zlib_state},
    },
    util::{
        hash::hash_file_crc32,
        native::{aeapp_dir, NativeError, SafeParent, SafeStr},
    },
};
use async_compression::tokio::write::DeflateDecoder;
use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use derive_getters::Getters;
use flate2::bufread::DeflateDecoder as BufreadDeflateDecoder;
use futures::{Stream, StreamExt, TryStreamExt};
use log::{debug, error, info, warn};
use reqwest::Client;
use strum_macros::Display;
use thiserror::Error;
use tokio::{
    fs::{create_dir, create_dir_all, File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    runtime::Handle,
};
use tokio_util::compat::FuturesAsyncReadCompatExt;

fn archive_entry_path(root: &Path, entry: &str) -> Result<PathBuf, DownloaderError> {
    let portable = entry.replace('\\', "/");
    let path = Path::new(&portable);
    use std::path::Component;
    if portable.is_empty()
        || portable.contains('\0')
        || portable.starts_with('/')
        || portable.as_bytes().get(1) == Some(&b':')
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !path
            .components()
            .any(|part| matches!(part, Component::Normal(_)))
    {
        return Err(DownloaderError::UnsafeArchivePath(entry.to_owned()));
    }
    Ok(root.join(path))
}

async fn checked_entry_path(root: &Path, entry: &str) -> Result<PathBuf, DownloaderError> {
    let path = archive_entry_path(root, entry)?;
    create_dir_all(root).await?;
    let canonical_root = tokio::fs::canonicalize(root).await?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| DownloaderError::UnsafeArchivePath(entry.to_owned()))?;
    let mut current = canonical_root.clone();
    for part in relative.components() {
        current.push(part.as_os_str());
        match tokio::fs::symlink_metadata(&current).await {
            Ok(_) => {
                let resolved = tokio::fs::canonicalize(&current).await?;
                if !resolved.starts_with(&canonical_root) {
                    return Err(DownloaderError::UnsafeArchivePath(entry.to_owned()));
                }
                current = resolved;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

async fn zstate_path(id: &str, path: &str) -> Result<PathBuf, DownloaderError> {
    if id.contains(['/', '\\']) {
        return Err(DownloaderError::UnsafeArchivePath(id.to_owned()));
    }
    // Validate both untrusted pieces before adding trusted cache prefixes.
    archive_entry_path(Path::new("/"), id)?;
    archive_entry_path(Path::new("/"), path)?;
    let mut relative = PathBuf::from("temp/downloader")
        .join(id)
        .join(path.replace('\\', "/"));
    relative.set_extension("eazstate");
    let path = checked_entry_path(&aeapp_dir()?, relative.safe_str()?).await?;
    create_dir_all(path.safe_parent()?).await?;
    Ok(path)
}

#[async_trait]
trait DownloadDecoder: Send {
    fn save_state(&mut self, buf: &mut BytesMut);
    fn restore_state(&mut self, buf: &mut Bytes);

    fn seek(&mut self, pos: SeekFrom) -> Result<(), DownloaderError>;

    fn write_in_pos(&self) -> u64;
    fn write_out_pos(&self) -> u64;

    fn get_mut<'b>(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>>;
}

struct ZLibDeflateDecoder {
    decoder: Arc<Mutex<DeflateDecoder<BufWriter<File>>>>,
}

impl ZLibDeflateDecoder {
    fn new(writer: BufWriter<File>) -> Self {
        Self {
            decoder: Arc::new(Mutex::new(DeflateDecoder::new(writer))),
        }
    }
}

#[async_trait]
impl DownloadDecoder for ZLibDeflateDecoder {
    fn save_state(&mut self, buf: &mut BytesMut) {
        let mut decoder = self.decoder.lock().unwrap();
        let zstream = decoder.inner_mut().decoder_mut().inner.decompress.get_raw();
        write_zlib_state(buf, zstream);
    }

    fn restore_state(&mut self, buf: &mut Bytes) {
        let mut decoder = self.decoder.lock().unwrap();
        let decompress = &mut decoder.inner_mut().decoder_mut().inner.decompress;
        decompress.reset(false);
        let zstream = decompress.get_raw();
        restore_zlib_state(buf, zstream);
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<(), DownloaderError> {
        let mut decoder = self.decoder.lock().unwrap();
        let file = decoder.get_mut();

        let handle = Handle::current();
        let _ = handle.enter();
        futures::executor::block_on(file.seek(pos))?;

        Ok(())
    }

    fn write_in_pos(&self) -> u64 {
        let mut decoder = self.decoder.lock().unwrap();
        let decompress = &mut decoder.inner_mut().decoder_mut().inner.decompress;
        let zstream = decompress.get_raw();
        zstream.total_in as u64
    }

    fn write_out_pos(&self) -> u64 {
        let mut decoder = self.decoder.lock().unwrap();
        let decompress = &mut decoder.inner_mut().decoder_mut().inner.decompress;
        let zstream = decompress.get_raw();
        zstream.total_out as u64
    }

    fn get_mut(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>> {
        self.decoder.clone()
    }
}

struct NoopDecoder {
    writer: Arc<Mutex<BufWriter<File>>>,
    pos: u64,
}

impl NoopDecoder {
    pub fn new(writer: BufWriter<File>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            pos: 0,
        }
    }
}

#[async_trait]
impl DownloadDecoder for NoopDecoder {
    fn save_state(&mut self, buf: &mut BytesMut) {
        self.pos = self.writer.lock().unwrap().buffer().len() as u64;
        buf.put_u64(self.pos);
    }

    fn restore_state(&mut self, buf: &mut Bytes) {
        self.pos = buf.get_u64();
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<(), DownloaderError> {
        let mut file = self.writer.lock().unwrap();

        let handle = Handle::current();
        let _ = handle.enter();
        futures::executor::block_on(file.seek(pos))?;

        Ok(())
    }

    fn write_in_pos(&self) -> u64 {
        self.pos
    }

    fn write_out_pos(&self) -> u64 {
        self.pos
    }

    fn get_mut<'b>(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>> {
        self.writer.clone()
    }
}

trait AsyncWriteWrapper: AsyncWrite + Unpin + Send {}
impl<T: AsyncWrite + Unpin + Send> AsyncWriteWrapper for T {}

struct AsyncWriterWrapper<'a> {
    id: String,
    path: String,
    zlib_state_file: std::fs::File,
    decoder: &'a mut Box<dyn DownloadDecoder>,
    inner: Arc<Mutex<dyn AsyncWriteWrapper>>,
}

impl<'a> AsyncWriterWrapper<'a> {
    async fn new(
        id: String,
        path: String,
        decoder: &'a mut Box<dyn DownloadDecoder>,
    ) -> Result<Self, DownloaderError> {
        let inner = decoder.get_mut();
        Ok(AsyncWriterWrapper {
            id: id.to_owned(),
            path: path.to_owned(),
            zlib_state_file: std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(zstate_path(&id, &path).await?)?,
            decoder,
            inner,
        })
    }
}

impl<'a> AsyncWrite for AsyncWriterWrapper<'a> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<prelude::v1::Result<usize, io::Error>> {
        let poll_result = {
            let mut binding = self.inner.lock().unwrap();
            let inner = Pin::new(&mut *binding);
            inner.poll_write(cx, buf)
        };

        // State serialization is disabled for now.
        // let mut bytes = BytesMut::new();
        // self.decoder.save_state(&mut bytes);

        // self.zlib_state_file.seek(SeekFrom::Start(0))?;
        // self.zlib_state_file.write(&bytes)?;

        poll_result
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<prelude::v1::Result<(), io::Error>> {
        Pin::new(&mut *self.inner.lock().unwrap()).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<prelude::v1::Result<(), io::Error>> {
        Pin::new(&mut *self.inner.lock().unwrap()).poll_shutdown(cx)
    }
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("download failed ({0} bytes")]
    DownloadFailed(usize),
    #[error("failed to download chunk `{entry}`: {error}")]
    ChunkDownload {
        entry: String,
        error: reqwest::Error,
    },
    #[error("failed to copy chunk `{entry}`: {error}")]
    ChunkCopy {
        entry: String,
        error: std::io::Error,
    },
}

#[derive(PartialEq, Debug)]
enum EntryDownloadState {
    Fresh,
    Resumable,
    Complete,
    Borked,
}

struct DownloadContext {
    id: String,
    path: PathBuf,
}

type BytesDownloadedCallback = Box<dyn Fn(usize) + Send + Sync>;

struct EntryDownloadRequest<'a> {
    context: &'a DownloadContext,
    url: &'a str,
    entry: &'a ZipFileEntry,
    client: Client,
    decoder: Box<dyn DownloadDecoder>,
    callback: Option<BytesDownloadedCallback>,
}

impl<'a> EntryDownloadRequest<'a> {
    pub fn new(
        context: &'a DownloadContext,
        url: &'a str,
        entry: &'a ZipFileEntry,
        client: Client,
        decoder: Box<dyn DownloadDecoder>,
        callback: Option<BytesDownloadedCallback>,
    ) -> Self {
        Self {
            context,
            url,
            entry,
            client,
            decoder,
            callback,
        }
    }

    async fn state(
        context: &DownloadContext,
        entry: &ZipFileEntry,
    ) -> Result<EntryDownloadState, DownloaderError> {
        let path = checked_entry_path(&context.path, entry.name()).await?;

        let file_size = File::open(&path).await?.metadata().await?.len() as i64;

        if file_size == 0 {
            return Ok(EntryDownloadState::Fresh);
        }

        let entry_size = *entry.uncompressed_size();
        let size_match = entry_size == file_size;

        if !size_match {
            warn!("Size mismatch: {}/{}", entry_size, file_size);
            if file_size > entry_size {
                return Ok(EntryDownloadState::Borked);
            }

            return Ok(EntryDownloadState::Borked);
        }

        // A cancelled transfer can have the expected file length. Verify content before
        // treating an existing file as complete instead of silently accepting corruption.
        let checked_path = path.clone();
        let hash = tokio::task::spawn_blocking(move || hash_file_crc32(checked_path))
            .await
            .map_err(|error| DownloaderError::Task(error.to_string()))??;
        if hash != *entry.crc32() {
            return Ok(EntryDownloadState::Borked);
        }

        Ok(EntryDownloadState::Complete)
    }

    async fn download(&mut self) -> Result<(), DownloaderError> {
        // Retrying with the same decompressor after replaying byte zero corrupts
        // output. Surface the error and let Retry recreate a fresh file/decoder.
        self.download_range(0, *self.entry.compressed_size()).await
    }

    /// End is not inclusive
    pub async fn download_range(&mut self, start: i64, end: i64) -> Result<(), DownloaderError> {
        let offset = self.entry.data_offset();
        let range = format!("bytes={}-{}", offset + start as i64, offset + end - 1);

        let data = match self
            .client
            .get(self.url)
            .header("range", range)
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("Failed to download ({}): {}", self.entry.name(), err);
                return Err(DownloaderError::Download(DownloadError::ChunkDownload {
                    entry: self.entry.name().clone(),
                    error: err,
                }));
            }
        };

        if data.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(DownloaderError::Http(data.status()));
        }
        let stream = data.bytes_stream();
        let counting_stream = ByteCountingStream::new(stream, self.callback.as_ref());
        let stream = counting_stream.into_async_read();
        let mut stream_reader = BufReader::new(stream.compat());

        // State deserialization is disabled for now.
        // let out_pos = self.decoder.write_out_pos();
        // self.decoder.seek(SeekFrom::Start(out_pos));

        let mut wrapper = AsyncWriterWrapper::new(
            self.context.id.to_owned(),
            self.entry.name().to_owned(),
            &mut self.decoder,
        )
        .await?;

        let result = tokio::io::copy(&mut stream_reader, &mut wrapper).await;
        if let Err(err) = result {
            return Err(DownloaderError::Download(DownloadError::ChunkCopy {
                entry: self.entry.name().clone(),
                error: err,
            }));
        }

        wrapper.shutdown().await?;
        Ok(())
    }
}

#[derive(Getters)]
pub struct ZipDownloader {
    id: String,
    url: String,
    path: PathBuf,
    client: Client,
    manifest: ZipFile,
}

async fn prepare_empty_entry(path: &Path, directory: bool) -> Result<(), DownloaderError> {
    if directory {
        create_dir_all(path).await?;
    } else {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await?;
    }
    Ok(())
}

impl ZipDownloader {
    pub async fn new<P: AsRef<Path>>(
        id: &str,
        zip_url: &str,
        path: P,
    ) -> Result<Self, DownloaderError>
    where
        PathBuf: From<P>,
    {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(DownloaderError::PathNotAbsolute(path));
        }

        let manifest = ZipFile::fetch(zip_url).await?;

        Ok(Self {
            id: id.to_owned(),
            url: zip_url.to_owned(),
            path,
            client: Client::builder().build()?,
            manifest,
        })
    }

    pub async fn read_zip_entry_bytes(
        &self,
        entry: &ZipFileEntry,
        length: u64,
    ) -> Result<Bytes, DownloaderError> {
        let offset = entry.data_offset();
        let compressed_size = *entry.compressed_size();

        let range_header = format!("bytes={}-{}", offset, offset + compressed_size - 1);

        let response = self
            .client
            .get(&self.url)
            .header("Range", range_header)
            .send()
            .await?;

        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            return Err(DownloaderError::Http(response.status()));
        }

        let compressed_data = response.bytes().await?;
        let decompressed_data = match entry.compression_type() {
            CompressionType::None => {
                let entry_size = *entry.uncompressed_size() as u64;
                let available_length = std::cmp::min(length, entry_size);

                if available_length > compressed_data.len() as u64 {
                    return Err(DownloaderError::EntrySize {
                        requested: available_length,
                        entry: compressed_data.len(),
                    });
                }

                Bytes::copy_from_slice(&compressed_data[..available_length as usize])
            }
            CompressionType::Deflate => {
                let mut decoder = BufreadDeflateDecoder::new(Cursor::new(&compressed_data));
                let mut limited_reader = decoder.take(length);
                let mut decompressed_data = Vec::with_capacity(length as usize);
                limited_reader.read_to_end(&mut decompressed_data)?;

                Bytes::from(decompressed_data)
            }
            any => {
                return Err(DownloaderError::CompressionType(any.to_owned()));
            }
        };

        Ok(decompressed_data)
    }

    pub async fn download_single_file(
        &self,
        entry: &ZipFileEntry,
        callback: Option<BytesDownloadedCallback>,
    ) -> Result<usize, DownloaderError> {
        let file_path = checked_entry_path(&self.path, entry.name()).await?;

        if entry.name().ends_with("/") {
            prepare_empty_entry(&file_path, true).await?;
            return Ok(0);
        }
        if *entry.uncompressed_size() == 0 {
            prepare_empty_entry(&file_path, false).await?;
            if let Some(callback) = callback {
                callback(*entry.compressed_size() as usize);
            }
            return Ok(0);
        }
        create_dir_all(file_path.safe_parent()?).await?;

        let offset = entry.data_offset();
        debug!("Type: {:?}", entry.compression_type());
        debug!("Compressed Size: {}", entry.compressed_size());
        debug!("Offset: {}", offset);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .await?;

        let context = DownloadContext {
            id: self.id.to_owned(),
            path: self.path.clone(),
        };

        let state = EntryDownloadRequest::state(&context, entry).await?;
        if state == EntryDownloadState::Complete {
            if let Some(callback) = callback {
                callback(*entry.compressed_size() as usize);
            }
            return Ok(0);
        }

        if state == EntryDownloadState::Borked {
            warn!("Found borked file {}", entry.name());
            file.set_len(0).await?;
        }

        let writer = tokio::io::BufWriter::new(file);

        let mut decoder: Box<dyn DownloadDecoder> = match entry.compression_type() {
            CompressionType::None => Box::new(NoopDecoder::new(writer)),
            CompressionType::Deflate => Box::new(ZLibDeflateDecoder::new(writer)),
        };

        if state == EntryDownloadState::Resumable {
            let state_file = zstate_path(&self.id, &entry.name()).await?;
            if state_file.exists() {
                let mut buf = Bytes::from(tokio::fs::read(state_file).await?);
                decoder.restore_state(&mut buf);
            } else {
                tokio::fs::create_dir_all(state_file.safe_parent()?).await?;
            }
        }

        let mut request = EntryDownloadRequest::new(
            &context,
            &self.url,
            entry,
            self.client.clone(),
            decoder,
            callback,
        );

        request.download().await?;
        let checked_path = file_path.clone();
        let hash = tokio::task::spawn_blocking(move || hash_file_crc32(checked_path))
            .await
            .map_err(|error| DownloaderError::Task(error.to_string()))??;
        if hash != *entry.crc32() {
            return Err(DownloaderError::Checksum(entry.name().clone()));
        }
        Ok(0)
    }
}

struct ByteCountingStream<'a, S> {
    inner: S,
    byte_count: usize,
    callback: Option<&'a BytesDownloadedCallback>,
}

impl<'a, S> ByteCountingStream<'a, S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
{
    fn new(inner: S, callback: Option<&'a BytesDownloadedCallback>) -> Self {
        ByteCountingStream {
            inner,
            byte_count: 0,
            callback,
        }
    }

    fn byte_count(&self) -> usize {
        self.byte_count
    }
}

impl<'a, S> Stream for ByteCountingStream<'a, S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<bytes::Bytes, tokio::io::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            std::task::Poll::Ready(Some(Ok(chunk))) => {
                self.byte_count += chunk.len();

                if let Some(callback) = &self.callback {
                    callback(chunk.len());
                }

                std::task::Poll::Ready(Some(Ok(chunk)))
            }
            std::task::Poll::Ready(Some(Err(err))) => {
                error!("Downloader error: {:?}", err);
                std::task::Poll::Ready(Some(Err(futures::io::Error::other(
                    DownloadError::DownloadFailed(self.byte_count),
                ))))
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn rejects_absolute_parent_and_windows_archive_paths() {
        let root = Path::new("/unused-install-root");
        for name in [
            "/etc/passwd",
            "../escape",
            "nested/../../escape",
            "\\server\\share",
            "C:\\escape",
            "C:/escape",
            "nested\\..\\escape",
            "",
            ".",
        ] {
            assert!(
                archive_entry_path(root, name).is_err(),
                "accepted unsafe path {name}"
            );
        }
        assert_eq!(
            archive_entry_path(root, "folder/game.bin").unwrap(),
            root.join("folder/game.bin")
        );
        assert_eq!(
            archive_entry_path(root, "folder\\game.bin").unwrap(),
            root.join("folder/game.bin")
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn existing_symlinks_cannot_escape_installation_or_cache_roots() {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("aeapp-path-check-{}", uuid::Uuid::new_v4())),
        );
        let root = scratch.0.join("install");
        let outside = scratch.0.join("outside");
        create_dir_all(&root).await.unwrap();
        create_dir_all(&outside).await.unwrap();
        tokio::fs::write(outside.join("untouched.txt"), b"unchanged")
            .await
            .unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
        assert!(matches!(
            checked_entry_path(&root, "escape/untouched.txt").await,
            Err(DownloaderError::UnsafeArchivePath(_))
        ));
        std::os::unix::fs::symlink(outside.join("untouched.txt"), root.join("file-link")).unwrap();
        assert!(checked_entry_path(&root, "file-link").await.is_err());
        let cache = scratch.0.join("cache");
        create_dir_all(&cache).await.unwrap();
        std::os::unix::fs::symlink(&outside, cache.join("temp")).unwrap();
        assert!(
            checked_entry_path(&cache, "temp/downloader/id/state.eazstate")
                .await
                .is_err()
        );
        assert_eq!(
            tokio::fs::read(outside.join("untouched.txt"))
                .await
                .unwrap(),
            b"unchanged"
        );
        let inside = root.join("actual");
        create_dir_all(&inside).await.unwrap();
        std::os::unix::fs::symlink(&inside, root.join("inside-link")).unwrap();
        assert!(checked_entry_path(&root, "inside-link/new-file")
            .await
            .is_ok());
    }
    #[tokio::test]
    async fn empty_entries_create_parent_folders_and_exact_empty_files() {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("aeapp-empty-entry-{}", uuid::Uuid::new_v4())),
        );
        let directory = scratch.0.join("nested/directory");
        prepare_empty_entry(&directory, true).await.unwrap();
        assert!(directory.is_dir());
        let empty = scratch.0.join("other/empty.dat");
        prepare_empty_entry(&empty, false).await.unwrap();
        assert!(empty.is_file());
        assert_eq!(tokio::fs::metadata(&empty).await.unwrap().len(), 0);
        tokio::fs::write(&empty, b"stale contents").await.unwrap();
        prepare_empty_entry(&empty, false).await.unwrap();
        assert_eq!(tokio::fs::metadata(&empty).await.unwrap().len(), 0);
        // Repeated folder entries must never be treated as files or removed.
        prepare_empty_entry(&directory, true).await.unwrap();
        assert!(directory.is_dir());
    }
}
