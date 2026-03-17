use std::borrow::Cow;
use std::path::Path;

use path_slash::PathExt;

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
