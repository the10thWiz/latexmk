//
// latex.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::collections::HashMap;

use crate::{recipe::Recipe, Options};

pub fn make_cmds(options: &Options, map: &mut HashMap<String, Recipe>) {
    // pdflatex
    map.insert(
        "pdf".into(),
        Recipe {
            uses: "tex",
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
            extras: &[],
            generated: &[],
            generated_dirs: &[],
            script: "dvilualatex --recorder --file-line-error --interaction=nonstopmode --synctex=1 \"%I\"".into(),
        },
    );
}

// TODO: Known latex warnings
//LaTeX Warning: There were undefined references.
//
//LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right.
//
//
