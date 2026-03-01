use std::io;

use crate::models::FileInfo;

pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn io_other<E: std::fmt::Display>(error: E) -> io::Error {
    io::Error::other(error.to_string())
}

pub(crate) fn sorted_files(files: &[FileInfo]) -> Vec<&FileInfo> {
    let mut refs = files.iter().collect::<Vec<_>>();
    refs.sort_by(|a, b| a.path.cmp(&b.path));
    refs
}
