use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;

use crate::api::ApiError;

const MAX_ARCHIVE_ENTRIES: usize = 4096;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 50 * 1024 * 1024;
const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 200 * 1024 * 1024;

struct ArchiveTracker {
    entries: usize,
    unpacked_bytes: u64,
}

impl ArchiveTracker {
    fn new() -> Self {
        Self {
            entries: 0,
            unpacked_bytes: 0,
        }
    }

    fn track_entry(&mut self, entry_size: u64) -> Result<(), ApiError> {
        self.entries += 1;
        if self.entries > MAX_ARCHIVE_ENTRIES {
            return Err(ApiError::validation_error("Archive contains too many entries"));
        }
        if entry_size > MAX_ARCHIVE_ENTRY_BYTES {
            return Err(ApiError::validation_error("Archive entry is too large"));
        }
        self.unpacked_bytes = self
            .unpacked_bytes
            .checked_add(entry_size)
            .ok_or_else(|| ApiError::validation_error("Archive is too large"))?;
        if self.unpacked_bytes > MAX_ARCHIVE_UNPACKED_BYTES {
            return Err(ApiError::validation_error("Archive uncompressed size is too large"));
        }
        Ok(())
    }
}

pub(crate) fn extract_zip(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| ApiError::validation_error(format!("Invalid ZIP: {}", e)))?;
    let mut tracker = ArchiveTracker::new();
    let mut root_name = None;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
        if file.is_symlink() {
            return Err(ApiError::validation_error(format!(
                "Archive symlink entry is not allowed: {}",
                file.name()
            )));
        }

        let safe_name = file.enclosed_name().ok_or_else(|| {
            ApiError::validation_error(format!("Unsafe ZIP entry path: {}", file.name()))
        })?;
        let safe_name = sanitize_archive_path(&safe_name)?;
        if root_name.is_none() {
            root_name = first_component_name(&safe_name);
        }

        tracker.track_entry(if file.is_dir() { 0 } else { file.size() })?;
        let outpath = dest.join(&safe_name);

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
        } else if file.is_file() {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| ApiError::internal_error(format!("File error: {}", e)))?;
            let copied = io::copy(&mut file, &mut outfile)
                .map_err(|e| ApiError::internal_error(format!("Write error: {}", e)))?;
            if copied != file.size() {
                return Err(ApiError::validation_error("Archive entry size mismatch"));
            }
            if copied > MAX_ARCHIVE_ENTRY_BYTES {
                return Err(ApiError::validation_error("Archive entry is too large"));
            }
        } else {
            return Err(ApiError::validation_error(format!(
                "Unsupported ZIP entry type: {}",
                file.name()
            )));
        }
    }

    root_name.ok_or_else(|| ApiError::validation_error("Empty archive"))
}

pub(crate) fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    extract_tar_inner(decoder, dest)
}

pub(crate) fn extract_tar(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    extract_tar_inner(cursor, dest)
}

fn extract_tar_inner<R: Read>(reader: R, dest: &Path) -> Result<String, ApiError> {
    let mut archive = Archive::new(reader);
    let mut tracker = ArchiveTracker::new();
    let mut root_name = None;

    for entry in archive
        .entries()
        .map_err(|e| ApiError::validation_error(format!("Invalid TAR: {}", e)))?
    {
        let mut entry = entry.map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
        let entry_type = entry.header().entry_type();
        if !(entry_type.is_file() || entry_type.is_dir()) {
            return Err(ApiError::validation_error("Archive special entries are not allowed"));
        }

        let path = entry
            .path()
            .map_err(|e| ApiError::internal_error(format!("Path error: {}", e)))?;
        let safe_name = sanitize_archive_path(&path)?;
        if root_name.is_none() {
            root_name = first_component_name(&safe_name);
        }

        let size = if entry_type.is_dir() { 0 } else { entry.header().size().unwrap_or(0) };
        tracker.track_entry(size)?;
        let outpath = dest.join(&safe_name);

        if entry_type.is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
            }
            entry
                .unpack(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Unpack error: {}", e)))?;
        }
    }

    root_name.ok_or_else(|| ApiError::validation_error("Empty archive"))
}

fn sanitize_archive_path(path: &Path) -> Result<PathBuf, ApiError> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part_text = part.to_string_lossy();
                if part_text.is_empty() || part_text.contains(':') {
                    return Err(ApiError::validation_error(format!(
                        "Unsafe archive entry path: {}",
                        path.display()
                    )));
                }
                clean.push(part);
            }
            Component::CurDir => {}
            _ => {
                return Err(ApiError::validation_error(format!(
                    "Unsafe archive entry path: {}",
                    path.display()
                )));
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(ApiError::validation_error("Empty archive entry path"));
    }
    Ok(clean)
}

fn first_component_name(path: &Path) -> Option<String> {
    path.components().find_map(|component| match component {
        Component::Normal(part) => Some(part.to_string_lossy().to_string()),
        _ => None,
    })
}

pub(crate) fn validate_package_dir_name(kind: &str, name: &str) -> Result<(), ApiError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(ApiError::validation_error(format!("Invalid {} name", kind)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_archive_paths() {
        assert!(sanitize_archive_path(Path::new("../plugin.json")).is_err());
        assert!(sanitize_archive_path(Path::new("/plugin.json")).is_err());
        assert!(sanitize_archive_path(Path::new("C:/plugin.json")).is_err());
    }

    #[test]
    fn accepts_normal_archive_paths() {
        let path = sanitize_archive_path(Path::new("plugin/plugin.json")).unwrap();
        assert_eq!(path, PathBuf::from("plugin").join("plugin.json"));
    }

    #[test]
    fn rejects_path_like_package_names() {
        assert!(validate_package_dir_name("plugin", "../demo").is_err());
        assert!(validate_package_dir_name("theme", "demo/theme").is_err());
        assert!(validate_package_dir_name("theme", "C:demo").is_err());
        assert!(validate_package_dir_name("plugin", "demo").is_ok());
    }
}
