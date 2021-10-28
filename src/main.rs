//! Latexmk like build tool
//!
//! latexmk supports way more options, but the defaults are good enough for most people.
//!
//! TODO:
//! - Support custom recipes (A few more options need to be added...)
//! - More builtin options
//! + Clean operation
//! - Log files allowing clean to avoid running all files, and potentially faster opteration?

use std::{path::PathBuf, str::FromStr};

//use structopt::{clap::Shell, StructOpt};
use clap::{Clap, IntoApp};
use clap_generate::{
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
    Shell,
};

mod job;
mod latex;
mod recipe;
mod sage;
mod util;

/// Command line tool to automatically build latex documents
#[derive(Debug, Clap)]
pub struct Options {
    /// Compile to dvi rather than pdf
    #[clap(short, long)]
    dvi: bool,
    /// Sets output file for itermediate files (TODO)
    #[clap(short, long, default_value = "./")]
    output_dir: String,
    /// Automatically clean up generated files
    ///
    /// Note that this still runs the full build process, since latexmk doesn't keep a log of the
    /// generated files between runs
    #[clap(short, long)]
    clean: bool,
    /// Files to compile [default: ./*.tex]
    files: Vec<PathBuf>,
    /// Output shell completion script
    ///
    /// Supported shells: [Bash, Zsh]
    /// Note that this overrides any other settings specified
    #[clap(long)]
    shell_completion: Option<Shell>,
}

fn main() -> std::io::Result<()> {
    let mut options = Options::parse();
    if let Some(shell) = options.shell_completion {
        match shell {
            Shell::Bash => clap_generate::generate::<Bash, _>(
                &mut Options::into_app(),
                std::env::current_exe()?
                    .file_name()
                    .map_or("latexmk", |f| f.to_str().unwrap_or("latexmk")),
                &mut std::io::stdout(),
            ),
            Shell::Elvish => clap_generate::generate::<Elvish, _>(
                &mut Options::into_app(),
                std::env::current_exe()?
                    .file_name()
                    .map_or("latexmk", |f| f.to_str().unwrap_or("latexmk")),
                &mut std::io::stdout(),
            ),
            Shell::Fish => clap_generate::generate::<Fish, _>(
                &mut Options::into_app(),
                std::env::current_exe()?
                    .file_name()
                    .map_or("latexmk", |f| f.to_str().unwrap_or("latexmk")),
                &mut std::io::stdout(),
            ),
            Shell::PowerShell => clap_generate::generate::<PowerShell, _>(
                &mut Options::into_app(),
                std::env::current_exe()?
                    .file_name()
                    .map_or("latexmk", |f| f.to_str().unwrap_or("latexmk")),
                &mut std::io::stdout(),
            ),
            Shell::Zsh => clap_generate::generate::<Zsh, _>(
                &mut Options::into_app(),
                std::env::current_exe()?
                    .file_name()
                    .map_or("latexmk", |f| f.to_str().unwrap_or("latexmk")),
                &mut std::io::stdout(),
            ),
            _ => todo!(),
        }
        return Ok(());
    }

    // Insert all files that end with .tex in the current directory if no files were specified
    if options.files.len() == 0 {
        let f = PathBuf::from_str(".").unwrap();
        for file in f.read_dir()? {
            let file = file?;
            if file.file_name().to_str().unwrap().ends_with(".tex") {
                options.files.push(file.path());
            }
        }
    }

    //recipe::run_cmds(options)
    job::run(options)
}
