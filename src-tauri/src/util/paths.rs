use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs::{self, DirBuilder, OpenOptions};
use std::path::{Path, PathBuf};

/// Make a directory that no other account can enter.
///
/// Tarcie's files hold the user's notes verbatim, and nothing in them was
/// encrypted or masked. They sit under the home directory, and whether that is
/// closed to other accounts is the distribution's choice rather than tarcie's:
/// a home at 0755 leaves every capture readable by anyone with a login.
///
/// The directory is the gate. A closed directory keeps other accounts out of
/// the files inside it whatever mode those files carry, which is what makes a
/// queue an earlier version created at 0644 safe without rewriting it.
///
/// An existing directory is closed as well as a new one, because creation does
/// not touch a directory that is already there.
#[cfg(unix)]
pub fn owner_only_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .with_context(|| format!("create {}", path.display()))?;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("close {}", path.display()))
}

#[cfg(not(unix))]
pub fn owner_only_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("create {}", path.display()))
}

/// Open options that create a file no other account can read.
///
/// The mode applies only where the file is created; an existing file keeps
/// what it has. That is the directory's job to cover.
#[cfg(unix)]
pub fn owner_only_file() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;

    let mut opts = OpenOptions::new();
    opts.mode(0o600);
    opts
}

#[cfg(not(unix))]
pub fn owner_only_file() -> OpenOptions {
    OpenOptions::new()
}

pub fn app_data_dir() -> Result<PathBuf> {
    let proj = ProjectDirs::from("com", "boswell", "tarcie")
        .ok_or_else(|| anyhow::anyhow!("failed to resolve project dirs"))?;
    Ok(proj.data_local_dir().to_path_buf())
}

pub fn queue_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("queue"))
}

pub fn sent_dir() -> Result<PathBuf> {
    Ok(queue_dir()?.join("sent"))
}

pub fn device_id_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("device_id.txt"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("logs"))
}

/// Where the day of the last scheduled delivery is recorded.
pub fn schedule_marker_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("last_scheduled_flush.txt"))
}
