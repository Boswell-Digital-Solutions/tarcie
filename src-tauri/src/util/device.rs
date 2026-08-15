use crate::util::log;
use crate::util::paths::device_id_path;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Load the device identity, minting one on first run.
///
/// Every event carries this ID, so the sink can tell one machine's captures
/// from another's. The ID is written down on the first run and read back on
/// every run after it.
pub fn load_or_create_device_id() -> Result<Uuid> {
    load_or_create_device_id_at(&device_id_path()?)
}

/// Load the device identity from an explicit path.
///
/// `load_or_create_device_id` resolves the path from the platform data dir.
/// This function takes it directly, so a caller can isolate the identity file
/// from the real user profile.
pub fn load_or_create_device_id_at(path: &Path) -> Result<Uuid> {
    if let Ok(existing) = fs::read_to_string(path) {
        if let Ok(id) = Uuid::parse_str(existing.trim()) {
            return Ok(id);
        }

        // The file is there but does not hold an ID. A fresh one is written
        // over it rather than failing the launch: a capture tool that refuses
        // to start costs more than the identity it could not read.
        log::write(format_args!(
            "device id at {} is unreadable, minting a new one",
            path.display()
        ));
    }

    let id = Uuid::new_v4();
    if let Some(parent) = path.parent() {
        crate::util::paths::owner_only_dir(parent).context("create the device id dir")?;
    }
    fs::write(path, id.to_string()).context("write the device id")?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_path() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("create temp dir");
        let path = dir.path().join("device_id.txt");
        (dir, path)
    }

    #[test]
    fn a_first_run_mints_an_id_and_writes_it_down() {
        let (_dir, path) = temp_path();

        let id = load_or_create_device_id_at(&path).expect("mint an id");

        assert!(path.exists(), "the id is persisted, not only returned");
        assert_eq!(
            fs::read_to_string(&path).expect("read it back").trim(),
            id.to_string(),
            "the file holds the id that was returned"
        );
    }

    #[test]
    fn a_later_run_returns_the_same_id() {
        let (_dir, path) = temp_path();

        let first = load_or_create_device_id_at(&path).expect("mint an id");
        let second = load_or_create_device_id_at(&path).expect("read the id");

        assert_eq!(first, second, "the identity survives a restart");
    }

    #[test]
    fn a_trailing_newline_is_tolerated() {
        // A file edited by hand, or written by a tool that adds a newline,
        // still reads as the same identity.
        let (_dir, path) = temp_path();
        let id = Uuid::new_v4();
        fs::write(&path, format!("  {id}\n")).expect("write the id");

        assert_eq!(
            load_or_create_device_id_at(&path).expect("read the id"),
            id,
            "surrounding whitespace does not cost the identity"
        );
    }

    #[test]
    fn an_unreadable_id_is_replaced_rather_than_failing_the_launch() {
        let (_dir, path) = temp_path();
        fs::write(&path, "this is not a uuid").expect("write junk");

        let id = load_or_create_device_id_at(&path).expect("mint over the junk");

        assert_eq!(
            fs::read_to_string(&path).expect("read it back").trim(),
            id.to_string(),
            "the junk is replaced by the new id"
        );
    }

    #[test]
    fn a_replaced_id_stays_stable_on_the_run_after_it() {
        // Minting over a damaged file is a one-time cost. If the replacement
        // were not written down, every run would mint again and the sink would
        // see one machine as many.
        let (_dir, path) = temp_path();
        fs::write(&path, "not a uuid").expect("write junk");

        let replacement = load_or_create_device_id_at(&path).expect("mint over the junk");
        let next_run = load_or_create_device_id_at(&path).expect("read the id");

        assert_eq!(replacement, next_run);
    }

    #[test]
    fn an_empty_file_is_treated_as_no_id_at_all() {
        let (_dir, path) = temp_path();
        fs::write(&path, "").expect("write an empty file");

        let id = load_or_create_device_id_at(&path).expect("mint an id");

        assert_eq!(
            fs::read_to_string(&path).expect("read it back").trim(),
            id.to_string()
        );
    }

    #[test]
    fn a_missing_parent_directory_is_created() {
        // The first run happens before anything has made the data dir.
        let dir = TempDir::new().expect("create temp dir");
        let path = dir.path().join("nested/deeper/device_id.txt");

        let id = load_or_create_device_id_at(&path).expect("mint an id");

        assert!(path.exists(), "the tree was created on the way");
        assert_eq!(
            fs::read_to_string(&path).expect("read it back").trim(),
            id.to_string()
        );
    }

    #[test]
    fn two_devices_do_not_share_an_id() {
        let (_one, first_path) = temp_path();
        let (_two, second_path) = temp_path();

        let first = load_or_create_device_id_at(&first_path).expect("mint an id");
        let second = load_or_create_device_id_at(&second_path).expect("mint an id");

        assert_ne!(first, second, "each profile gets its own identity");
    }
}
