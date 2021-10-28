//
// sage.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::collections::HashMap;

use crate::{recipe::Recipe, Options};

pub fn make_cmds(_options: &Options, map: &mut HashMap<String, Recipe>) {
    // sage
    map.insert(
        "sagetex.sout".into(),
        Recipe {
            uses: "sagetex.sage",
            extras: &[],
            generated: &["sagetex.sage.py", "sagetex.scmd"],
            generated_dirs: &["sage-plots-for-"],
            script: "sage \"%I\"".into(),
        },
    );
}
