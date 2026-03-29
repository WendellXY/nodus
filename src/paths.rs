use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use path_slash::PathExt;

pub fn canonicalize_path(path: &Path) -> io::Result<PathBuf> {
    dunce::canonicalize(path)
}

pub fn strip_path_prefix<'a>(path: &'a Path, base: &Path) -> Option<&'a Path> {
    dunce::simplified(path)
        .strip_prefix(dunce::simplified(base))
        .ok()
}

pub fn path_is_dir(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_dir())
        .unwrap_or_else(|_| {
            canonicalize_path(path)
                .map(|canonical| canonical.is_dir())
                .unwrap_or(false)
        })
}

pub fn display_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".into()
    } else {
        match path.to_slash_lossy() {
            Cow::Borrowed(value) => value.to_string(),
            Cow::Owned(value) => value,
        }
    }
}
