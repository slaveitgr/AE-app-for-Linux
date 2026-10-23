//! Read-only preflight for a Windows archive; no game files are created here.
use super::{downloader::ZipDownloader, ContentService};
use std::path::{Path, PathBuf};
#[cfg(not(unix))]
use sysinfo::{DiskExt, System, SystemExt};

#[derive(Clone, Debug)]
pub struct InstallSpace {
    pub build_id: String,
    pub required: u64,
    pub available: u64,
}

fn sum_sizes(mut sizes: impl Iterator<Item = i64>) -> Result<u64, String> {
    sizes.try_fold(0_u64, |total, size| {
        total
            .checked_add(u64::try_from(size).map_err(|_| "Invalid archive size")?)
            .ok_or_else(|| "Archive size exceeds supported limits".to_owned())
    })
}

#[cfg(any(not(unix), test))]
fn closest_mount<'a>(path: &Path, disks: impl Iterator<Item = (&'a Path, u64)>) -> Option<u64> {
    disks
        .filter(|(mount, _)| path.starts_with(mount))
        .max_by_key(|(mount, _)| mount.components().count())
        .map(|(_, space)| space)
}

pub fn available_install_space(path: &Path) -> Result<u64, String> {
    if !path.is_absolute() || path.parent().is_none() {
        return Err("Choose an absolute game folder inside a writable drive.".into());
    }
    let existing = path
        .ancestors()
        .find(|p| p.exists())
        .ok_or("The installation drive could not be found.")?;
    if !existing.is_dir() {
        return Err("The installation folder points to a file.".into());
    }
    let existing = existing
        .canonicalize()
        .map_err(|_| "The installation folder is not accessible.")?;
    filesystem_available_space(&existing)
}

#[cfg(unix)]
fn filesystem_available_space(path: &Path) -> Result<u64, String> {
    use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt};
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "The folder contains an invalid character.")?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: path is NUL terminated and stats points to writable storage. The
    // result is read only after statvfs reports successful initialization.
    let stats = unsafe {
        if libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) != 0 {
            return Err("Available disk space could not be read for this folder.".into());
        }
        stats.assume_init()
    };
    if stats.f_flag & libc::ST_RDONLY as libc::c_ulong != 0 {
        return Err("The selected installation drive is read-only.".into());
    }
    (stats.f_bavail as u64)
        .checked_mul(stats.f_frsize as u64)
        .ok_or_else(|| "The drive reported an unsupported available-space value.".into())
}

#[cfg(not(unix))]
fn filesystem_available_space(path: &Path) -> Result<u64, String> {
    let mut system = System::new();
    system.refresh_disks_list();
    closest_mount(
        path,
        system
            .disks()
            .iter()
            .map(|disk| (disk.mount_point(), disk.available_space())),
    )
    .ok_or_else(|| "Available disk space could not be read for this drive.".into())
}

pub async fn prepare_install(
    service: &ContentService,
    offer: &str,
    path: PathBuf,
) -> Result<InstallSpace, String> {
    available_install_space(&path)?;
    let build = service
        .windows_live_build(offer)
        .await
        .map_err(|error| match error {
            crate::core::service_layer::ServiceLayerError::GamePlatform(reason) => reason,
            _ => "Windows build metadata could not be loaded. Try again.".into(),
        })?;
    let url = service
        .download_url(offer, Some(build.build_id()))
        .await
        .map_err(|_| "The Windows download could not be prepared. Try again.")?;
    let archive = ZipDownloader::new(offer, url.url(), &path)
        .await
        .map_err(|_| "The archive size could not be read. Try again.")?;
    let required = sum_sizes(
        archive
            .manifest()
            .entries()
            .iter()
            .map(|entry| *entry.uncompressed_size()),
    )?;
    let available = available_install_space(&path)?;
    Ok(InstallSpace {
        build_id: build.build_id().to_owned(),
        required,
        available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_and_overflowing_archive_sizes_are_rejected() {
        assert_eq!(sum_sizes([0, 2048, 4096].into_iter()).unwrap(), 6144);
        assert!(sum_sizes([-1].into_iter()).is_err());
        assert!(sum_sizes([i64::MAX, i64::MAX, 2].into_iter()).is_err());
    }
    #[test]
    fn nested_install_drive_uses_its_own_free_space() {
        let drives = [(Path::new("/"), 100), (Path::new("/mnt/games"), 20)];
        assert_eq!(
            closest_mount(Path::new("/mnt/games/EA/Apex"), drives.into_iter()),
            Some(20)
        );
        assert_eq!(
            closest_mount(Path::new("/mnt/games-other/Apex"), drives.into_iter()),
            Some(100)
        );
        assert!(available_install_space(Path::new("relative/game")).is_err());
        assert!(available_install_space(Path::new("/")).is_err());
    }
    #[test]
    fn a_new_game_folder_can_be_checked_without_creating_it() {
        let path = std::env::temp_dir()
            .join(format!("aeapp-space-check-{}", std::process::id()))
            .join("game");
        assert!(!path.exists());
        assert!(available_install_space(&path).is_ok());
        assert!(!path.exists());
    }
}
