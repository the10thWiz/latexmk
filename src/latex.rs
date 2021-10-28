//
// latex.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::{collections::HashMap, io::Write, process::Command};

use crate::{
    recipe::Recipe,
    util::{file_error, replace_file_ext},
    Options,
};

pub fn make_cmds(_options: &Options, map: &mut HashMap<String, Recipe>) {
    // pdflatex
    map.insert(
        "pdf".into(),
        Recipe {
            uses: "tex",
            f: &|_, _, _| Ok(()),
            extras: &[],
            generated: &["fls", "synctex.gz"],
            generated_dirs: &[],
            script:
                "pdflatex -recorder -file-line-error -interaction nonstopmode -synctex 1 \"%I\""
                    .into(),
        },
    );
    // dvilualatex
    map.insert(
        "dvi".into(),
        Recipe {
            uses: "tex",
            f: &|_, _, _| Ok(()),
            extras: &[],
            generated: &[],
            generated_dirs: &[],
            script: "dvilualatex --recorder --file-line-error --interaction=nonstopmode --synctex=1 \"%I\"".into(),
        },
    );
}

pub fn recipes(_options: &Options, map: &mut HashMap<String, crate::job::Recipe>) {
    map.insert(
        "pdf".into(),
        crate::job::Recipe {
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
                if cmd.status.success() {
                    queue.output(file.clone());
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "log"));
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "aux"));
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "fls"));
                    Ok(())
                } else {
                    std::io::stdout().write_all(&cmd.stdout)?;
                    std::io::stdout().write_all(&cmd.stderr)?;
                    Err(file_error("Sage error"))
                }
            },
            needs_to_run: &|_, _| true,
        },
    );
    map.insert(
        "dvi".into(),
        crate::job::Recipe {
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
                    .output()?;
                if cmd.status.success() {
                    queue.output(file.clone());
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "log"));
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "aux"));
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "fls"));
                    queue.output(replace_file_ext(queue.tex_file(), "tex", "synctex.gz"));
                    Ok(())
                } else {
                    std::io::stdout().write_all(&cmd.stdout)?;
                    std::io::stdout().write_all(&cmd.stderr)?;
                    Err(file_error("Sage error"))
                }
            },
            needs_to_run: &|_, _| true,
        },
    );
}

// TODO: Known latex warnings
//LaTeX Warning: There were undefined references.
//
//LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right.
//
//
