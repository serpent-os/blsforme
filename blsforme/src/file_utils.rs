// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Case-insensitive path joining for FAT, respecting existing entries on the filesystem
//! Note, this discards errors, so will require read permissions
use std::{
    fs,
    path::{Path, PathBuf},
};

pub trait PathExt<P: AsRef<Path>> {
    fn join_insensitive(&self, path: P) -> PathBuf;
}

impl<P: AsRef<Path>> PathExt<P> for PathBuf {
    fn join_insensitive(&self, path: P) -> PathBuf {
        let real_path: &Path = path.as_ref();
        if let Ok(dir) = fs::read_dir(self) {
            let entries = dir.filter_map(|e| e.ok()).filter_map(|p| {
                let n = p.file_name();
                n.into_string().ok()
            });
            for entry in entries {
                if entry.to_lowercase() == real_path.to_string_lossy().to_lowercase() {
                    return self.join(&entry);
                }
            }
        }
        self.join(path)
    }
}
