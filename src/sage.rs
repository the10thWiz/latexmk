//
// sage.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    recipe::Recipe,
    util::{file_error, replace_file_ext},
    Options,
};

pub fn recipes(_options: &Options, map: &mut HashMap<String, Recipe>) {
    map.insert(
        "sagetex.sout".into(),
        Recipe {
            uses: "sagetex.sage",
            run: &|file, queue| {
                println!("Running Sage on {}", file.display());
                let cmd = Command::new("sage")
                    .arg(replace_file_ext(file, "sagetex.sout", "sagetex.sage"))
                    .output()?;
                if cmd.status.success() {
                    queue.output(file.clone());
                    queue.output(replace_file_ext(file, "sagetex.sout", "sagetex.sage.py"));
                    queue.output(replace_file_ext(file, "sagetex.sout", "sagetex.scmd"));
                    queue.output(queue.tex_file().with_file_name(format!(
                        "sage-plots-for-{}",
                        queue.tex_file().file_name().map_or("", |f| f.to_str().unwrap_or(""))
                    )));
                    Ok(())
                } else {
                    std::io::stdout().write_all(&cmd.stdout)?;
                    std::io::stdout().write_all(&cmd.stderr)?;
                    Err(file_error("Sage error"))
                }
            },
            needs_to_run: &|file, _queue| {
                let sage = replace_file_ext(file, "sagetex.sout", "sagetex.sage");
                match sage_digest(sage).map(|d| sage_digest_check(file, d)) {
                    Ok(Ok(val)) => val,
                    _ => true,
                }
            },
        },
    );
}

/// Returns the digest of a sage file without a specific set of lines
fn sage_digest(file: impl AsRef<Path>) -> std::io::Result<String> {
    let file = File::open(file)?;
    let mut hash = md5::Context::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.starts_with("_st_.goboom")
            && !trimmed.starts_with("print('SageT")
            && !trimmed.starts_with(" ?_st_.current_tex_line")
        {
            hash.consume(line.as_bytes());
        }
    }
    Ok(dbg!(format!("%{:x}% md5sum", hash.compute())))
}

/// Checks if file contains the digest, and returns false if such a line is found
fn sage_digest_check(file: impl AsRef<Path>, digest: String) -> std::io::Result<bool> {
    let file = File::open(file)?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.starts_with(&digest) {
            return Ok(false);
        }
    }
    Ok(true)
}
