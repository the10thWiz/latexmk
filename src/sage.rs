//
// sage.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::{collections::HashMap, io::Write, path::PathBuf, process::Command};

use crate::{
    recipe::Recipe,
    util::{file_error, replace_file_ext},
    Options,
};

pub fn make_cmds(_options: &Options, map: &mut HashMap<String, Recipe>) {
    // sage
    map.insert(
        "sagetex.sout".into(),
        Recipe {
            uses: "sagetex.sage",
            f: &|_, _, _| Ok(()),
            extras: &[],
            generated: &["sagetex.sage.py", "sagetex.scmd"],
            generated_dirs: &["sage-plots-for-"],
            script: "sage \"%I\"".into(),
        },
    );
}

pub fn recipes(_options: &Options, map: &mut HashMap<String, crate::job::Recipe>) {
    map.insert(
        "sagetex.sout".into(),
        crate::job::Recipe {
            uses: "sagetex.sage",
            f: &|file, queue| {
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
        },
    );
}
