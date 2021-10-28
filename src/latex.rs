//
// latex.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

use crate::{
    job::JobQueue,
    recipe::Recipe,
    util::{file_error, replace_file_ext},
    Options,
};

pub fn recipes(_options: &Options, map: &mut HashMap<String, Recipe>) {
    map.insert(
        "pdf".into(),
        Recipe {
            uses: "tex",
            run: &|file, queue| {
                println!("Running pdflatex on {}", file.display());
                let cmd = Command::new("pdflatex")
                    .arg("-recorder")
                    .arg("-file-line-error")
                    .arg("-interaction")
                    .arg("nonstopmode")
                    .arg("-synctex")
                    .arg("1")
                    .arg(queue.tex_file())
                    .output()?;
                run_latex(file, queue, cmd)
            },
            needs_to_run: &|_, _| true,
        },
    );
    map.insert(
        "dvi".into(),
        Recipe {
            uses: "tex",
            run: &|file, queue| {
                println!("Running dvilualatex on {}", file.display());
                let cmd = Command::new("dvilualatex")
                    .arg("--recorder")
                    .arg("--file-line-error")
                    .arg("--interaction")
                    .arg("nonstopmode")
                    .arg("--synctex")
                    .arg("1")
                    .arg(queue.tex_file())
                    .stdout(Stdio::piped())
                    .output()?;
                run_latex(file, queue, cmd)
            },
            needs_to_run: &|_, _| true,
        },
    );
}

/// Runs the shared portion - checking the fls file, checking the output / log, etc
fn run_latex(file: &PathBuf, queue: &mut JobQueue, cmd: Output) -> std::io::Result<()> {
    queue.output(file.clone());
    queue.output(replace_file_ext(queue.tex_file(), "tex", "log"));
    queue.output(replace_file_ext(queue.tex_file(), "tex", "aux"));
    queue.output(replace_file_ext(queue.tex_file(), "tex", "fls"));
    queue.output(replace_file_ext(queue.tex_file(), "tex", "synctex.gz"));
    collect_files(replace_file_ext(queue.tex_file(), "tex", "fls"), queue)?;
    let stdout = String::from_utf8(cmd.stdout).map_err(|_| file_error("Non-utf8 error"))?;
    for file in find(&stdout) {
        queue.needs(PathBuf::from_str(&file).unwrap());
    }
    if check_warnings(&stdout) {
        queue.rerun();
    }
    if cmd.status.success() {
        Ok(())
    } else {
        std::io::stdout().write_all(stdout.as_bytes())?;
        std::io::stdout().write_all(&cmd.stderr)?;
        Err(file_error("Sage error"))
    }
}

fn collect_files(flsfile: impl AsRef<Path>, deps: &mut JobQueue) -> std::io::Result<()> {
    let mut r = File::open(flsfile)?;
    let mut s = String::new();
    r.read_to_string(&mut s)?;
    let mut pwd = PathBuf::from_str(".").unwrap();
    for line in s.split('\n').filter(|s| s.trim() != "") {
        let (cmd, file) = line
            .trim()
            .split_once(' ')
            .ok_or(file_error("no space found"))?;
        let mut path = PathBuf::from_str(file).map_err(|_| file_error("not a valid path"))?;
        // make absolute if possible
        if !path.is_absolute() {
            path = pwd.join(path);
        }
        // Handle various possiblilities
        if cmd == "PWD" {
            pwd = path;
        } else if cmd == "INPUT" {
            //deps.input.insert(path);
            deps.needs(path);
        } else if cmd == "OUTPUT" {
            //deps.output.insert(path);
            deps.output(path);
        } else {
            panic!("Unexpected line: {}", cmd);
        }
    }
    Ok(())
}

/// Find `No file ` in outputs
fn find(s: &str) -> HashSet<String> {
    let mut ret = HashSet::new();
    let mut cur = s;
    while cur.len() > 0 {
        if let Some((_pre, rest)) = cur.split_once("No file ") {
            let filename = rest.split_once('\n').map_or(rest, |(r, _)| r);
            ret.insert(filename[..filename.len() - 1].into());
            cur = &rest[1..];
        } else {
            break;
        }
    }
    ret
}

fn check_warnings(s: &str) -> bool {
    s.contains("LaTeX Warning: Label(s) may have changed")
        || s.contains("LaTeX Warning: There were undefined references")
}
