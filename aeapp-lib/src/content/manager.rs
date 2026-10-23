use crate::{
    content::{
        downloader::{DownloadError, ZipDownloader},
        zip::{CompressionType, ZipError, ZipFileEntry},
        ContentService,
    },
    core::{
        auth::storage::LockedAuthStorage,
        manifest::{self, ManifestError, MANIFEST_RELATIVE_PATH},
        service_layer::ServiceLayerError,
        AeAppEvent,
    },
    util::native::{aeapp_dir, NativeError},
};
use derive_builder::Builder;
use derive_getters::Getters;
use futures::{FutureExt, StreamExt, TryStreamExt};
use log::{debug, info};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use thiserror::Error;
use tokio::{
    fs,
    io::AsyncWriteExt,
    sync::{watch, Notify},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
const QUEUE_FILE: &str = "download_queue.json";

#[derive(Default, Builder, Getters, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuedGame {
    offer_id: String,
    build_id: String,
    path: PathBuf,
}

#[derive(Default, Getters, Serialize, Deserialize)]
pub struct DownloadQueue {
    current: Option<QueuedGame>,
    paused: bool,
    queued: Vec<QueuedGame>,
    completed: Vec<QueuedGame>,
    #[serde(default)]
    last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadPhase {
    #[default]
    Idle,
    Downloading,
    Paused,
    Finalizing,
    Finished,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone)]
pub enum DownloadAction {
    Pause(String),
    Resume(String),
    Cancel(String),
    RemoveQueued(String),
    MoveToFront(String),
}
#[derive(Debug, Clone, Default)]
pub struct DownloadStatus {
    pub offer_id: Option<String>,
    pub phase: DownloadPhase,
    pub bytes: usize,
    pub total: usize,
    pub error: Option<String>,
}

#[derive(Error, Debug)]
pub enum ContentManagerError {
    #[error(transparent)]
    Downloader(#[from] DownloaderError),
    #[error(transparent)]
    Native(#[from] NativeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("download in progress, you must cancel it before starting a new one")]
    DownloadInProgress,
    #[error("this game is already downloading or queued")]
    AlreadyQueued,
    #[error("the download is no longer in the queue")]
    DownloadMissing,
    #[error("download queue is invalid at {path}; the file was preserved: {error}")]
    CorruptQueue {
        path: PathBuf,
        error: serde_json::Error,
    },
    #[error("the installation is finishing; wait for it to complete before changing it")]
    Finalizing,
}
#[derive(Error, Debug)]
pub enum DownloaderError {
    #[error(transparent)]
    ServiceLayer(#[from] ServiceLayerError),
    #[error(transparent)]
    Zip(#[from] ZipError),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Native(#[from] NativeError),
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("path `{0}` is not absolute")]
    PathNotAbsolute(PathBuf),
    #[error("failed to download range: {0}")]
    Http(StatusCode),
    #[error("requested length ({requested}) exceeds entry size ({entry})")]
    EntrySize { requested: u64, entry: usize },
    #[error("unsupported compression type `{0:?}`")]
    CompressionType(CompressionType),
    #[error("download cancelled")]
    Cancelled,
    #[error("download worker failed: {0}")]
    Task(String),
    #[error("downloaded file failed its checksum: {0}")]
    Checksum(String),
    #[error("archive path is outside the installation or download cache: {0}")]
    UnsafeArchivePath(String),
}

impl DownloadQueue {
    pub(crate) async fn load() -> Result<Self, ContentManagerError> {
        Self::load_file(&aeapp_dir()?.join(QUEUE_FILE)).await
    }
    async fn load_file(file: &Path) -> Result<Self, ContentManagerError> {
        let data = match fs::read_to_string(file).await {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(error.into()),
        };
        serde_json::from_str(&data).map_err(|error| ContentManagerError::CorruptQueue {
            path: file.to_owned(),
            error,
        })
    }
    pub(crate) async fn save(&self) -> Result<(), ContentManagerError> {
        self.save_file(&aeapp_dir()?.join(QUEUE_FILE)).await
    }
    async fn save_file(&self, destination: &Path) -> Result<(), ContentManagerError> {
        let temporary =
            destination.with_file_name(format!(".download-queue-{}.tmp", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec(self)?;
        let result = async {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await?;
            file.write_all(&bytes).await?;
            file.sync_all().await?;
            drop(file);
            fs::rename(&temporary, destination).await?;
            Ok::<(), ContentManagerError>(())
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&temporary).await;
        }
        result
    }
    pub fn push_to_current(&mut self, game: QueuedGame) {
        if let Some(current) = self.current.replace(game) {
            self.queued.push(current);
        }
    }
    fn contains(&self, offer: &str) -> bool {
        self.current.as_ref().is_some_and(|g| g.offer_id == offer)
            || self.queued.iter().any(|g| g.offer_id == offer)
    }
    fn remove_queued(&mut self, offer: &str) -> Result<QueuedGame, ContentManagerError> {
        let index = self
            .queued
            .iter()
            .position(|g| g.offer_id == offer)
            .ok_or(ContentManagerError::DownloadMissing)?;
        Ok(self.queued.remove(index))
    }
    fn move_to_front(&mut self, offer: &str) -> Result<(), ContentManagerError> {
        let game = self.remove_queued(offer)?;
        self.queued.insert(0, game);
        Ok(())
    }
    fn pop_next(&mut self) -> Option<QueuedGame> {
        if self.queued.is_empty() {
            None
        } else {
            Some(self.queued.remove(0))
        }
    }
}

/// A paused transfer retains its pinned I/O future, decoder and open file in memory.
/// Cancellation drops those futures before installation finalization can begin.
#[derive(Clone)]
struct DownloadControl {
    phase: Arc<Mutex<DownloadPhase>>,
    paused: watch::Sender<bool>,
    cancel: CancellationToken,
}
impl DownloadControl {
    fn new() -> Self {
        let (paused, _) = watch::channel(false);
        Self {
            phase: Arc::new(Mutex::new(DownloadPhase::Downloading)),
            paused,
            cancel: CancellationToken::new(),
        }
    }
    fn phase(&self) -> DownloadPhase {
        *self.phase.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn pause(&self) -> Result<(), ContentManagerError> {
        let mut phase = self.phase.lock().unwrap_or_else(|e| e.into_inner());
        match *phase {
            DownloadPhase::Downloading | DownloadPhase::Paused => {
                *phase = DownloadPhase::Paused;
                self.paused.send_replace(true);
                Ok(())
            }
            _ => Err(ContentManagerError::Finalizing),
        }
    }
    fn resume(&self) {
        let mut phase = self.phase.lock().unwrap_or_else(|e| e.into_inner());
        if *phase == DownloadPhase::Paused {
            *phase = DownloadPhase::Downloading;
            self.paused.send_replace(false);
        }
    }
    fn cancel(&self) -> Result<(), ContentManagerError> {
        let mut phase = self.phase.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(*phase, DownloadPhase::Finalizing | DownloadPhase::Finished) {
            return Err(ContentManagerError::Finalizing);
        }
        self.cancel.cancel();
        *phase = DownloadPhase::Cancelled;
        self.paused.send_replace(false);
        Ok(())
    }
    async fn run<F: Future>(&self, future: F) -> Result<F::Output, DownloaderError> {
        let mut paused = self.paused.subscribe();
        tokio::pin!(future);
        loop {
            loop {
                let is_paused = *paused.borrow_and_update();
                if !is_paused {
                    break;
                }
                tokio::select! { biased;
                    _=self.cancel.cancelled()=>return Err(DownloaderError::Cancelled),
                    changed=paused.changed()=>{ if changed.is_err(){return Err(DownloaderError::Cancelled);} }
                }
            }
            tokio::select! { biased;
                _=self.cancel.cancelled()=>return Err(DownloaderError::Cancelled),
                changed=paused.changed()=>{ if changed.is_err(){return Err(DownloaderError::Cancelled);} },
                result=&mut future=>return Ok(result),
            }
        }
    }
    async fn begin_finalizing(&self) -> Result<(), DownloaderError> {
        loop {
            self.run(std::future::ready(())).await?;
            let mut phase = self.phase.lock().unwrap_or_else(|e| e.into_inner());
            if self.cancel.is_cancelled() {
                return Err(DownloaderError::Cancelled);
            }
            if *phase == DownloadPhase::Paused {
                continue;
            }
            *phase = DownloadPhase::Finalizing;
            return Ok(());
        }
    }
    fn complete(&self, result: &Result<(), DownloaderError>) {
        let mut phase = self.phase.lock().unwrap_or_else(|e| e.into_inner());
        *phase = match result {
            Ok(()) => DownloadPhase::Finished,
            Err(DownloaderError::Cancelled) => DownloadPhase::Cancelled,
            Err(_) => DownloadPhase::Failed,
        };
    }
}

async fn catch_download_task<F>(future: F) -> Result<(), DownloaderError>
where
    F: Future<Output = Result<(), DownloaderError>>,
{
    match std::panic::AssertUnwindSafe(future).catch_unwind().await {
        Ok(result) => result,
        Err(_) => Err(DownloaderError::Task(
            "transfer worker panicked; Retry can restart it safely".to_owned(),
        )),
    }
}

pub struct GameDownloader {
    offer_id: String,
    downloader: Arc<ZipDownloader>,
    entries: Vec<ZipFileEntry>,
    control: DownloadControl,
    completed_bytes: Arc<AtomicUsize>,
    total_bytes: usize,
    notify: Arc<Notify>,
    task: Option<JoinHandle<()>>,
    error: Arc<Mutex<Option<String>>>,
}
impl GameDownloader {
    pub async fn new(service: &ContentService, game: &QueuedGame) -> Result<Self, DownloaderError> {
        let url = service
            .download_url(&game.offer_id, Some(&game.build_id))
            .await?;
        debug!("Preparing game download for {}", game.offer_id);
        let downloader = ZipDownloader::new(&game.offer_id, &url.url(), &game.path).await?;
        let entries: Vec<_> = downloader.manifest().entries().to_vec();
        let total_bytes = entries.iter().map(|x| *x.compressed_size() as usize).sum();
        Ok(Self {
            offer_id: game.offer_id.clone(),
            downloader: Arc::new(downloader),
            entries,
            control: DownloadControl::new(),
            completed_bytes: Arc::new(AtomicUsize::new(0)),
            total_bytes,
            notify: Arc::new(Notify::new()),
            task: None,
            error: Arc::new(Mutex::new(None)),
        })
    }
    pub fn download(&mut self) {
        let downloader = self.downloader.clone();
        let entries = self.entries.clone();
        let control = self.control.clone();
        let bytes = self.completed_bytes.clone();
        let notify = self.notify.clone();
        let error = self.error.clone();
        self.task = Some(tokio::spawn(async move {
            let result = catch_download_task(Self::start_downloads(
                downloader,
                entries,
                control.clone(),
                bytes,
            ))
            .await;
            if let Err(err) = &result {
                *error.lock().unwrap_or_else(|e| e.into_inner()) = Some(err.to_string());
            }
            control.complete(&result);
            notify.notify_waiters();
        }));
    }
    async fn start_downloads(
        downloader: Arc<ZipDownloader>,
        entries: Vec<ZipFileEntry>,
        control: DownloadControl,
        bytes: Arc<AtomicUsize>,
    ) -> Result<(), DownloaderError> {
        futures::stream::iter(entries.into_iter().map(|entry| {
            let downloader = downloader.clone();
            let control = control.clone();
            let bytes = bytes.clone();
            async move {
                let future = downloader.download_single_file(
                    &entry,
                    Some(Box::new(move |amount| {
                        bytes.fetch_add(amount, Ordering::SeqCst);
                    })),
                );
                control.run(future).await??;
                Ok::<(), DownloaderError>(())
            }
        }))
        .buffer_unordered(16)
        .try_collect::<Vec<_>>()
        .await?;
        control.begin_finalizing().await?;
        info!("Files downloaded, running installation setup...");
        let path = downloader.path();
        let manifest = manifest::read(path.join(MANIFEST_RELATIVE_PATH)).await?;
        manifest.run_touchup(path).await?;
        info!("Installation finished");
        Ok(())
    }
    async fn cancel_and_wait(&mut self) -> Result<(), ContentManagerError> {
        self.control.cancel()?;
        if let Some(task) = self.task.take() {
            // Even an unexpected join failure means the transfer task has stopped.
            // Cancellation can therefore clear its queue item instead of stranding it.
            if let Err(error) = task.await {
                log::warn!("Stopped transfer worker: {error}");
            }
        }
        Ok(())
    }
    pub fn cancel(&self) {
        let _ = self.control.cancel();
    }
    pub async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if matches!(
                self.control.phase(),
                DownloadPhase::Finished | DownloadPhase::Failed | DownloadPhase::Cancelled
            ) {
                return;
            }
            notified.await;
        }
    }
    pub fn is_done(&self) -> bool {
        self.control.phase() == DownloadPhase::Finished
    }
    pub fn percentage_done(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_downloaded() as f64 / self.total_bytes as f64 * 100.0).min(100.0)
        }
    }
    pub fn bytes_downloaded(&self) -> usize {
        self.completed_bytes
            .load(Ordering::SeqCst)
            .min(self.total_bytes)
    }
    pub fn bytes_total(&self) -> usize {
        self.total_bytes
    }
    pub fn offer_id(&self) -> &String {
        &self.offer_id
    }
    pub fn status(&self) -> DownloadStatus {
        DownloadStatus {
            offer_id: Some(self.offer_id.clone()),
            phase: self.control.phase(),
            bytes: self.bytes_downloaded(),
            total: self.total_bytes,
            error: self.error.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        }
    }
}
impl Drop for GameDownloader {
    fn drop(&mut self) {
        self.control.cancel.cancel();
    }
}

#[derive(Getters)]
pub struct ContentManager {
    queue: DownloadQueue,
    service: ContentService,
    current: Option<GameDownloader>,
}
impl ContentManager {
    pub async fn new(auth: LockedAuthStorage, _resume: bool) -> Result<Self, ContentManagerError> {
        let mut queue = DownloadQueue::load().await?;
        // Recover a queue written between tasks without silently starting network I/O.
        if queue.current.is_none() {
            queue.current = queue.pop_next();
            queue.last_error = None;
        }
        // Restarting the client does not silently begin a potentially large transfer.
        if queue.current.is_some() {
            queue.paused = true;
        }
        Ok(Self {
            queue,
            service: ContentService::new(auth),
            current: None,
        })
    }
    pub fn download_status(&self) -> DownloadStatus {
        self.current
            .as_ref()
            .map(GameDownloader::status)
            .unwrap_or_else(|| DownloadStatus {
                offer_id: self.queue.current.as_ref().map(|g| g.offer_id.clone()),
                phase: if self.queue.current.is_some() {
                    if self.queue.last_error.is_some() {
                        DownloadPhase::Failed
                    } else {
                        DownloadPhase::Paused
                    }
                } else {
                    DownloadPhase::Idle
                },
                error: self.queue.last_error.clone(),
                ..DownloadStatus::default()
            })
    }
    pub async fn add_install(&mut self, game: QueuedGame) -> Result<(), ContentManagerError> {
        if self.queue.contains(&game.offer_id) {
            return Err(ContentManagerError::AlreadyQueued);
        }
        if self.queue.current.is_none() && self.current.is_none() && self.queue.queued.is_empty() {
            self.install_now(game).await?;
        } else {
            self.queue.queued.push(game);
            self.queue.save().await?;
        }
        Ok(())
    }
    pub async fn install_now(&mut self, game: QueuedGame) -> Result<(), ContentManagerError> {
        if let Some(current) = self.current.as_mut() {
            current.cancel_and_wait().await?;
        }
        self.current = None;
        if let Some(old) = self.queue.current.take() {
            if old.offer_id != game.offer_id {
                self.queue.queued.push(old);
            }
        }
        self.queue
            .queued
            .retain(|queued| queued.offer_id != game.offer_id);
        self.install_direct(game).await
    }
    async fn install_direct(&mut self, game: QueuedGame) -> Result<(), ContentManagerError> {
        if self.current.is_some() {
            return Err(ContentManagerError::DownloadInProgress);
        }
        self.queue.current = Some(game.clone());
        self.queue.paused = true;
        self.queue.last_error = None;
        self.queue.save().await?;
        let mut downloader = match GameDownloader::new(&self.service, &game).await {
            Ok(downloader) => downloader,
            Err(error) => {
                self.queue.last_error = Some(error.to_string());
                self.queue.save().await?;
                return Err(error.into());
            }
        };
        downloader.download();
        self.current = Some(downloader);
        self.queue.paused = false;
        self.queue.save().await?;
        Ok(())
    }
    async fn start_next(&mut self) -> Result<(), ContentManagerError> {
        if let Some(game) = self.queue.pop_next() {
            self.install_direct(game).await?;
        } else {
            self.queue.save().await?;
        }
        Ok(())
    }
    pub async fn control_download(
        &mut self,
        action: DownloadAction,
    ) -> Result<(), ContentManagerError> {
        match action {
            DownloadAction::RemoveQueued(offer) => {
                self.queue.remove_queued(&offer)?;
                self.queue.save().await?;
            }
            DownloadAction::MoveToFront(offer) => {
                self.queue.move_to_front(&offer)?;
                self.queue.save().await?;
            }
            DownloadAction::Pause(offer) => {
                if self
                    .queue
                    .current
                    .as_ref()
                    .is_none_or(|g| g.offer_id != offer)
                {
                    return Err(ContentManagerError::DownloadMissing);
                }
                self.current
                    .as_ref()
                    .ok_or(ContentManagerError::DownloadMissing)?
                    .control
                    .pause()?;
                self.queue.paused = true;
                self.queue.save().await?;
            }
            DownloadAction::Resume(offer) => {
                let game = self
                    .queue
                    .current
                    .as_ref()
                    .filter(|g| g.offer_id == offer)
                    .cloned()
                    .ok_or(ContentManagerError::DownloadMissing)?;
                if let Some(current) = &self.current {
                    match current.control.phase() {
                        DownloadPhase::Paused => current.control.resume(),
                        DownloadPhase::Failed | DownloadPhase::Cancelled => {
                            self.current = None;
                            self.install_direct(game).await?;
                        }
                        DownloadPhase::Downloading => {}
                        _ => return Err(ContentManagerError::Finalizing),
                    }
                } else {
                    self.install_direct(game).await?;
                }
                self.queue.paused = false;
                self.queue.save().await?;
            }
            DownloadAction::Cancel(offer) => {
                if self
                    .queue
                    .current
                    .as_ref()
                    .is_none_or(|g| g.offer_id != offer)
                {
                    return Err(ContentManagerError::DownloadMissing);
                }
                if let Some(current) = self.current.as_mut() {
                    current.cancel_and_wait().await?;
                }
                self.current = None;
                self.queue.current = None;
                self.queue.paused = false;
                self.queue.last_error = None;
                self.start_next().await?;
            }
        }
        Ok(())
    }
    pub(crate) async fn update(&mut self) -> Result<Option<AeAppEvent>, ContentManagerError> {
        if let Some(current) = &self.current {
            if current.control.phase() == DownloadPhase::Failed {
                let error = current
                    .error
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if self.queue.last_error != error {
                    self.queue.last_error = error;
                    self.queue.paused = true;
                    self.queue.save().await?;
                }
            }
        }
        if self.current.as_ref().is_some_and(GameDownloader::is_done) {
            let offer = self
                .current
                .as_ref()
                .map(|g| g.offer_id.clone())
                .unwrap_or_default();
            self.current = None;
            if let Some(game) = self.queue.current.take() {
                self.queue.completed.push(game);
            }
            self.queue.paused = false;
            self.queue.last_error = None;
            // Preserve the completion event even if preparing the next game fails.
            if let Err(err) = self.start_next().await {
                log::warn!("Next download is paused: {err}");
            }
            return Ok(Some(AeAppEvent::InstallFinished(offer)));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(id: &str) -> QueuedGame {
        QueuedGame {
            offer_id: id.to_owned(),
            build_id: "test".into(),
            path: PathBuf::from("/unused-test-path"),
        }
    }
    #[test]
    fn queued_downloads_use_fifo_and_reorder_without_losing_entries() {
        let mut queue = DownloadQueue {
            queued: vec![game("a"), game("b"), game("c")],
            ..Default::default()
        };
        queue.move_to_front("c").unwrap();
        assert_eq!(queue.pop_next().unwrap().offer_id, "c");
        assert_eq!(queue.pop_next().unwrap().offer_id, "a");
        assert_eq!(queue.pop_next().unwrap().offer_id, "b");
        assert!(queue.pop_next().is_none());
    }
    #[test]
    fn removing_unknown_download_preserves_queue() {
        let mut queue = DownloadQueue {
            queued: vec![game("a")],
            ..Default::default()
        };
        assert!(queue.remove_queued("missing").is_err());
        assert_eq!(queue.queued.len(), 1);
        assert_eq!(queue.remove_queued("a").unwrap().offer_id, "a");
    }
    #[test]
    fn persisted_queue_retains_pause_and_order() {
        let queue = DownloadQueue {
            current: Some(game("current")),
            paused: true,
            queued: vec![game("a"), game("b")],
            completed: vec![],
            last_error: Some("offline test failure".to_owned()),
        };
        let restored: DownloadQueue =
            serde_json::from_str(&serde_json::to_string(&queue).unwrap()).unwrap();
        assert!(restored.paused);
        assert_eq!(restored.last_error.as_deref(), Some("offline test failure"));
        assert_eq!(restored.current.unwrap().offer_id, "current");
        assert_eq!(restored.queued[0].offer_id, "a");
    }
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[tokio::test]
    async fn queue_files_replace_atomically_and_corruption_is_preserved() {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("aeapp-queue-test-{}", uuid::Uuid::new_v4())),
        );
        fs::create_dir_all(&scratch.0).await.unwrap();
        let path = scratch.0.join("queue.json");
        let mut queue = DownloadQueue {
            current: Some(game("first")),
            ..Default::default()
        };
        queue.save_file(&path).await.unwrap();
        queue.current = Some(game("second"));
        queue.save_file(&path).await.unwrap();
        assert_eq!(
            DownloadQueue::load_file(&path)
                .await
                .unwrap()
                .current
                .unwrap()
                .offer_id,
            "second"
        );
        let mut entries = fs::read_dir(&scratch.0).await.unwrap();
        let mut count = 0;
        while entries.next_entry().await.unwrap().is_some() {
            count += 1;
        }
        assert_eq!(
            count, 1,
            "successful atomic writes leave no temporary files"
        );
        let malformed = b"{broken queue";
        fs::write(&path, malformed).await.unwrap();
        assert!(matches!(
            DownloadQueue::load_file(&path).await,
            Err(ContentManagerError::CorruptQueue { .. })
        ));
        assert_eq!(fs::read(&path).await.unwrap(), malformed);
    }
    #[tokio::test]
    async fn worker_panic_becomes_retryable_failure() {
        let control = DownloadControl::new();
        let result = catch_download_task(async { panic!("offline worker panic fixture") }).await;
        assert!(matches!(result, Err(DownloaderError::Task(_))));
        control.complete(&result);
        assert_eq!(control.phase(), DownloadPhase::Failed);
        assert!(control.cancel().is_ok());
    }
    #[tokio::test]
    async fn pause_does_not_poll_transfer_and_resume_continues_same_future() {
        let control = DownloadControl::new();
        control.pause().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        let worker = control.clone();
        let task = tokio::spawn(async move {
            worker
                .run(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    42
                })
                .await
        });
        tokio::task::yield_now().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
        control.resume();
        assert_eq!(task.await.unwrap().unwrap(), 42);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn pause_retains_a_partially_polled_transfer() {
        let control = DownloadControl::new();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let gate = Arc::new(Notify::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let (worker, worker_gate, worker_counter) =
            (control.clone(), gate.clone(), counter.clone());
        let task = tokio::spawn(async move {
            worker
                .run(async move {
                    worker_counter.fetch_add(1, Ordering::SeqCst);
                    let _ = started_tx.send(());
                    worker_gate.notified().await;
                    worker_counter.fetch_add(1, Ordering::SeqCst);
                    7
                })
                .await
        });
        started_rx.await.unwrap();
        control.pause().unwrap();
        gate.notify_one();
        tokio::task::yield_now().await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        control.resume();
        assert_eq!(task.await.unwrap().unwrap(), 7);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
    #[tokio::test]
    async fn cancelling_paused_transfer_prevents_finalization() {
        let control = DownloadControl::new();
        control.pause().unwrap();
        control.cancel().unwrap();
        assert!(matches!(
            control.run(std::future::ready(())).await,
            Err(DownloaderError::Cancelled)
        ));
        assert!(matches!(
            control.begin_finalizing().await,
            Err(DownloaderError::Cancelled)
        ));
    }
    #[tokio::test]
    async fn finalization_cannot_be_interrupted() {
        let control = DownloadControl::new();
        control.begin_finalizing().await.unwrap();
        assert!(matches!(
            control.pause(),
            Err(ContentManagerError::Finalizing)
        ));
        assert!(matches!(
            control.cancel(),
            Err(ContentManagerError::Finalizing)
        ));
        assert!(!control.cancel.is_cancelled());
    }
}
