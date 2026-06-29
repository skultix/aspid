//! Zip archive extraction helpers, used for both mod and modding-API installs.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Extract every file in a zip archive into `dest`, preserving the internal directory
/// structure. Returns the list of written file paths (relative to `dest`).
///
/// Entries with unsafe paths (absolute or `..` traversal) are skipped via
/// [`zip::read::ZipFile::enclosed_name`].
pub fn extract_all(bytes: &[u8], dest: &Path) -> Result<Vec<PathBuf>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    std::fs::create_dir_all(dest).map_err(|e| Error::io(dest, e))?;

    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(rel) = entry.enclosed_name() else {
            continue; // unsafe path; skip
        };
        let out = dest.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| Error::io(&out, e))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| Error::io(&out, e))?;
        std::fs::write(&out, &buf).map_err(|e| Error::io(&out, e))?;
        written.push(rel.to_path_buf());
    }
    Ok(written)
}

/// Extract selected files from a zip, flattening them directly into `dest`.
///
/// `keep` is called with each entry's base file name; only matching files are written.
/// Used for the modding API, whose archive is a flat set of assemblies to drop into
/// `Managed/`. Returns the base names written.
pub fn extract_flat<F>(bytes: &[u8], dest: &Path, mut keep: F) -> Result<Vec<String>>
where
    F: FnMut(&str) -> bool,
{
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    std::fs::create_dir_all(dest).map_err(|e| Error::io(dest, e))?;

    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let Some(name) = rel.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if !keep(&name) {
            continue;
        }
        let out = dest.join(&name);
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| Error::io(&out, e))?;
        std::fs::write(&out, &buf).map_err(|e| Error::io(&out, e))?;
        written.push(name);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a tiny in-memory zip for tests.
    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            for (name, data) in entries {
                w.start_file(*name, opts).unwrap();
                w.write_all(data).unwrap();
            }
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn extract_all_preserves_structure() {
        let zip = make_zip(&[("ModName/ModName.dll", b"dll"), ("ModName/data.txt", b"x")]);
        let tmp = tempfile::tempdir().unwrap();
        let written = extract_all(&zip, tmp.path()).unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            std::fs::read(tmp.path().join("ModName/ModName.dll")).unwrap(),
            b"dll"
        );
    }

    #[test]
    fn extract_flat_filters_and_flattens() {
        let zip = make_zip(&[
            ("api/Assembly-CSharp.dll", b"asm"),
            ("api/README.md", b"readme"),
        ]);
        let tmp = tempfile::tempdir().unwrap();
        let written = extract_flat(&zip, tmp.path(), |n| n.ends_with(".dll")).unwrap();
        assert_eq!(written, vec!["Assembly-CSharp.dll"]);
        assert!(tmp.path().join("Assembly-CSharp.dll").exists());
        assert!(!tmp.path().join("README.md").exists());
    }
}
