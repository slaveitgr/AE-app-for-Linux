//! Native Linux StatusNotifierItem integration without GTK.
//! Only a registered tray with a supported restore path may hide the window.
use std::sync::{mpsc, Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayEvent {
    Open,
    Settings,
    Exit,
    Unavailable,
}

#[derive(Default)]
struct Availability {
    ready: bool,
    generation: u64,
    stopped: bool,
}
struct Shared {
    state: Mutex<Availability>,
    events: mpsc::Sender<TrayEvent>,
    context: egui::Context,
    #[cfg(target_os = "linux")]
    x11_restore: Mutex<Option<mpsc::SyncSender<()>>>,
}
impl Shared {
    fn event(&self, event: TrayEvent) {
        let _ = self.events.send(event);
        #[cfg(target_os = "linux")]
        if let Ok(restore) = self.x11_restore.lock() {
            if let Some(sender) = restore.as_ref() {
                match sender.try_send(()) {
                    Ok(()) | Err(mpsc::TrySendError::Full(())) => return,
                    Err(mpsc::TrySendError::Disconnected(())) => {}
                }
            }
        }
        self.context.request_repaint();
    }
    fn generation(&self) -> u64 {
        self.state.lock().map(|state| state.generation).unwrap_or(u64::MAX)
    }
    fn registered(&self, generation: u64) -> bool {
        let ready = if let Ok(mut state) = self.state.lock() {
            if state.generation != generation || state.stopped {
                return false;
            }
            state.ready = true;
            true
        } else {
            false
        };
        self.context.request_repaint();
        ready
    }
    fn unavailable(&self) {
        let notify = if let Ok(mut state) = self.state.lock() {
            let was_ready = state.ready && !state.stopped;
            state.ready = false;
            state.generation = state.generation.wrapping_add(1);
            was_ready
        } else {
            true
        };
        if notify {
            self.event(TrayEvent::Unavailable);
        }
    }
    fn stop(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.stopped = true;
            state.ready = false;
        }
    }
    fn can_hide(&self) -> bool {
        let ready = self.state.lock().map(|state| state.ready && !state.stopped).unwrap_or(false);
        if !ready {
            return false;
        }
        #[cfg(target_os = "linux")]
        {
            // Native Wayland cannot hide a winit 0.29 window. Registration alone
            // must not make its no-op Visible(false) look like a tray close.
            self.x11_restore.lock().map(|restore| restore.is_some()).unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        false
    }
}

pub struct NativeTray {
    events: mpsc::Receiver<TrayEvent>,
    shared: Arc<Shared>,
    #[cfg(target_os = "linux")]
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    #[cfg(target_os = "linux")]
    worker: tokio::task::JoinHandle<()>,
}
impl NativeTray {
    /// Returns immediately. Readiness stays false until strict watcher registration succeeds.
    /// Preview and unsupported platforms never connect to the session bus.
    pub fn start(context: &egui::Context, preview: bool) -> Option<Self> {
        if preview {
            return None;
        }
        #[cfg(target_os = "linux")]
        {
            let runtime = tokio::runtime::Handle::try_current().ok()?;
            let (sender, events) = mpsc::channel();
            let shared = Arc::new(Shared {
                state: Mutex::new(Availability::default()),
                events: sender,
                context: context.clone(),
                x11_restore: Mutex::new(None),
            });
            let (shutdown, stop) = tokio::sync::oneshot::channel();
            let worker = runtime.spawn(linux::run(shared.clone(), stop));
            Some(Self {
                events,
                shared,
                shutdown: Some(shutdown),
                worker,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = context;
            None
        }
    }
    /// Set the application's own XID from CreationContext, once, before tray use.
    /// A bounded worker restores it before requesting the repaint that drains tray events.
    pub fn set_x11_window(&self, window: u32) {
        #[cfg(target_os = "linux")]
        if window != 0 {
            if let Ok(mut restore) = self.shared.x11_restore.lock() {
                if restore.is_none() {
                    let (sender, receiver) = mpsc::sync_channel(1);
                    let context = self.shared.context.clone();
                    if std::thread::Builder::new()
                        .name("aeapp-tray-restore".to_owned())
                        .spawn(move || {
                            while receiver.recv().is_ok() {
                                if let Err(error) = linux::restore_window(window) {
                                    log::debug!("Could not restore AeApp's X11 window: {error}");
                                }
                                context.request_repaint();
                            }
                        })
                        .is_ok()
                    {
                        *restore = Some(sender);
                    }
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = window;
    }
    /// A false result means closing must minimize instead of hiding to the tray.
    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "linux")]
        if self.worker.is_finished() {
            return false;
        }
        self.shared.can_hide()
    }
    pub fn try_recv(&self) -> Option<TrayEvent> {
        self.events.try_recv().ok()
    }
}
impl Drop for NativeTray {
    fn drop(&mut self) {
        self.shared.stop();
        #[cfg(target_os = "linux")]
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{Shared, TrayEvent};
    use ksni::TrayMethods;
    use std::{sync::Arc, time::Duration};

    /// Use an independent X11 connection so a minimized egui viewport need not render first.
    pub(super) fn restore_window(window: u32) -> anyhow::Result<()> {
        use x11rb::{
            connection::Connection,
            protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask},
        };
        let (connection, _) = x11rb::connect(None)?;
        let pid_atom = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        let pid = connection
            .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut values| values.next());
        anyhow::ensure!(
            pid == Some(std::process::id()),
            "XID no longer belongs to AeApp"
        );
        let root = connection.get_geometry(window)?.reply()?.root;
        let active = connection.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
        connection.map_window(window)?.check()?;
        connection
            .send_event(
                false,
                root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ClientMessageEvent::new(32, window, active, [1, x11rb::CURRENT_TIME, 0, 0, 0]),
            )?
            .check()?;
        connection.flush()?;
        Ok(())
    }

    #[derive(Clone)]
    struct AppTray {
        shared: Arc<Shared>,
        icons: Vec<ksni::Icon>,
    }
    impl ksni::Tray for AppTray {
        fn id(&self) -> String {
            "io.github.slaveitgr.aeapp".to_owned()
        }
        fn title(&self) -> String {
            "aeapp".to_owned()
        }
        fn status(&self) -> ksni::Status {
            ksni::Status::Active
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            self.icons.clone()
        }
        fn activate(&mut self, _: i32, _: i32) {
            self.shared.event(TrayEvent::Open);
        }
        fn secondary_activate(&mut self, _: i32, _: i32) {
            self.shared.event(TrayEvent::Open);
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip {
                title: "aeapp".to_owned(),
                description: "Open aeapp".to_owned(),
                ..Default::default()
            }
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            vec![
                ksni::menu::StandardItem {
                    label: "Open aeapp".to_owned(),
                    activate: Box::new(|tray: &mut Self| tray.shared.event(TrayEvent::Open)),
                    ..Default::default()
                }
                .into(),
                ksni::menu::StandardItem {
                    label: "Settings".to_owned(),
                    activate: Box::new(|tray: &mut Self| tray.shared.event(TrayEvent::Settings)),
                    ..Default::default()
                }
                .into(),
                ksni::MenuItem::Separator,
                ksni::menu::StandardItem {
                    label: "Exit".to_owned(),
                    activate: Box::new(|tray: &mut Self| tray.shared.event(TrayEvent::Exit)),
                    ..Default::default()
                }
                .into(),
            ]
        }
        fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
            log::debug!("System tray watcher went offline: {reason:?}");
            self.shared.unavailable();
            // A fresh strict registration must succeed before close can hide again.
            false
        }
    }
    fn argb(mut rgba: Vec<u8>) -> Vec<u8> {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        rgba
    }
    fn icons() -> Vec<ksni::Icon> {
        // application_icon prefers the user's locally imported original EA icon.
        let icon = crate::application_icon();
        let Some(image) = image::RgbaImage::from_raw(icon.width, icon.height, icon.rgba) else {
            return Vec::new();
        };
        let image = image::DynamicImage::ImageRgba8(image);
        [16, 22, 24, 32, 48]
            .into_iter()
            .map(|size| {
                let image = image
                    .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                    .into_rgba8();
                ksni::Icon {
                    width: size as i32,
                    height: size as i32,
                    data: argb(image.into_raw()),
                }
            })
            .collect()
    }
    pub(super) async fn run(shared: Arc<Shared>, mut stop: tokio::sync::oneshot::Receiver<()>) {
        let tray = AppTray {
            shared: shared.clone(),
            icons: icons(),
        };
        loop {
            let generation = shared.generation();
            let result = tokio::select! {
                _ = &mut stop => break,
                result = tray.clone().spawn() => result,
            };
            match result {
                Ok(handle) => {
                    if !handle.is_closed() && shared.registered(generation) {
                        log::debug!("Native system tray registered");
                        loop {
                            tokio::select! {
                                _ = &mut stop => { handle.shutdown().await; shared.stop(); return; },
                                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                                    if handle.is_closed() { break; }
                                },
                            }
                        }
                    }
                    handle.shutdown().await;
                    shared.unavailable();
                }
                Err(error) => {
                    log::debug!("Native system tray is unavailable: {error}");
                    shared.unavailable();
                }
            }
            tokio::select! {
                _ = &mut stop => break,
                _ = tokio::time::sleep(Duration::from_secs(2)) => {},
            }
        }
        shared.stop();
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn status_notifier_pixels_are_argb_bytes() {
            assert_eq!(
                super::argb(vec![10, 20, 30, 255, 1, 2, 3, 0]),
                vec![255, 10, 20, 30, 0, 1, 2, 3]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watcher_loss_invalidates_pending_registration_and_requests_restore() {
        let (events, receiver) = mpsc::channel();
        let shared = Shared {
            state: Mutex::new(Availability::default()),
            events,
            context: egui::Context::default(),
            #[cfg(target_os = "linux")]
            x11_restore: Mutex::new(None),
        };
        let old = shared.generation();
        assert!(shared.registered(old));
        shared.unavailable();
        assert_eq!(receiver.try_recv().unwrap(), TrayEvent::Unavailable);
        assert!(!shared.registered(old));
        assert!(!shared.state.lock().unwrap().ready);
        assert!(shared.registered(shared.generation()));
        shared.stop();
        assert!(!shared.registered(shared.generation()));
    }
    #[test]
    fn preview_never_starts_a_native_tray() {
        assert!(NativeTray::start(&egui::Context::default(), true).is_none());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn registration_requires_a_restore_path_before_hiding() {
        let (events, _) = mpsc::channel();
        let shared = Shared {
            state: Mutex::new(Availability::default()),
            events,
            context: egui::Context::default(),
            x11_restore: Mutex::new(None),
        };
        assert!(shared.registered(shared.generation()));
        assert!(!shared.can_hide());

        let (sender, _receiver) = mpsc::sync_channel(1);
        *shared.x11_restore.lock().unwrap() = Some(sender);
        assert!(shared.can_hide());
        shared.unavailable();
        assert!(!shared.can_hide());
        assert!(shared.registered(shared.generation()));
        assert!(shared.can_hide());
        shared.stop();
        assert!(!shared.can_hide());
    }
}
