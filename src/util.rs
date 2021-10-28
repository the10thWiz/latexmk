use std::{
    io::Error,
    path::{Path, PathBuf},
};

//
// util.rs
// Copyright (C) 2021 matthew <matthew@WINDOWS-05HIC4F>
// Distributed under terms of the MIT license.
//

pub fn file_error(e: &'static str) -> Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}

pub fn replace_file_ext(path: &Path, cur_ext: &str, new_ext: &str) -> PathBuf {
    match path.file_name() {
        Some(name) => match name.to_str() {
            Some(s) => {
                if s.ends_with(cur_ext) {
                    return path.with_file_name(format!(
                        "{}{}",
                        &s[..s.len() - cur_ext.len()],
                        new_ext
                    ));
                }
            }
            _ => (),
        },
        _ => (),
    }
    path.to_path_buf()
}
