//! Latexmk like build tool
//!
//! latexmk supports way more options, but the defaults are good enough for most people.
//!
//! TODO:
//! - Support custom recipes (A few more options need to be added...)
//! - More builtin options
//! + Clean operation
//! - Log files allowing clean to avoid running all files, and potentially faster opteration?

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs::File,
    io::{Error, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

//use structopt::{clap::Shell, StructOpt};
use clap::{Clap, IntoApp};
use clap_generate::{
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
    Shell,
};

/// Command line tool to automatically build latex documents
#[derive(Debug, Clap)]
struct Options {
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

fn make_cmds() -> HashMap<String, Recipe> {
    let mut map = HashMap::new();
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
    // bibtex
    map.insert(
        "bbl".into(),
        Recipe {
            uses: "aux",
            extras: &["bib"],
            generated: &["blg"],
            generated_dirs: &[],
            script: "bibtex \"%N\"".into(),
        },
    );
    // use make
    map
}

#[derive(Debug, Default)]
struct Deps {
    input: HashSet<PathBuf>,
    output: HashSet<PathBuf>,
    missing: HashSet<String>,
}

impl Deps {
    fn clean(&mut self) {
        self.input.clear();
        self.missing.clear();
    }
}

struct Recipe {
    uses: &'static str,
    extras: &'static [&'static str],
    generated: &'static [&'static str],
    generated_dirs: &'static [&'static str],
    script: Cow<'static, str>,
}

fn with_parent<W>(path: &Path, f: impl FnOnce(&Path) -> W) -> W {
    if let Some(p) = path.parent() {
        if p.exists() {
            f(p)
        } else {
            f(&PathBuf::from_str(".").unwrap())
        }
    } else {
        f(&PathBuf::from_str(".").unwrap())
    }
}

impl Recipe {
    fn check_file_times(&self, input_name: &PathBuf, output_name: &str) -> std::io::Result<bool> {
        // Check file times and only rebuild if needed
        let output_time = File::open(input_name.with_file_name(output_name))?
            .metadata()?
            .modified()?;
        let input_time = File::open(input_name)?.metadata()?.modified()?;
        for path in PathBuf::from_str(".").unwrap().read_dir()? {
            let path = path?;
            let name = path.file_name();
            let name = name.to_str().unwrap_or("");
            for extra in self.extras.iter() {
                if name.ends_with(extra) {
                    if output_time > path.metadata()?.modified()? {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(output_time > input_time)
    }

    fn on_file(
        &self,
        path: &PathBuf,
        ext: &str,
        output: &mut HashSet<PathBuf>,
    ) -> std::io::Result<Output> {
        if let Ok(dir) = with_parent(path, |f| f.read_dir()) {
            for file in dir {
                if let Ok(file) = file {
                    if file.file_type().map_or(false, |f| f.is_dir()) {
                        let name = file.file_name();
                        let name = name.to_str().unwrap_or("");
                        if self.generated_dirs.iter().any(|gen| name.starts_with(gen)) {
                            output.insert(file.path());
                        }
                    } else {
                        let name = file.file_name();
                        let name = name.to_str().unwrap_or("");
                        if self.generated.iter().any(|gen| name.ends_with(gen)) {
                            output.insert(file.path());
                        }
                    }
                }
            }
        }
        let output_name = path.file_name().map_or("", |o| o.to_str().unwrap_or(""));
        let input_name = format!(
            "{}.{}",
            &output_name[..output_name.len() - ext.len() - 1],
            self.uses
        );
        println!("Running rule on {}", input_name);

        // Note that this function will fail with an error if the file doesn't exist, but there
        // is not harm is rebuilding the file if we don't need to.
        if matches!(self.check_file_times(&path, &output_name), Ok(true))
            || !path.with_file_name(&input_name).exists()
        {
            return Command::new("true").output();
        }

        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(
            self.script
                .replace("%O", output_name)
                .replace("%I", &input_name)
                .replace("%N", &output_name[..output_name.len() - ext.len() - 1])
                .replace("%%", "%"),
        );
        if let Some(parent) = path.parent() {
            if let Ok(dir) = parent.canonicalize() {
                cmd.current_dir(dir);
            }
        }
        cmd.stdout(Stdio::piped()).output()
    }

    fn run_for(&self, path: &PathBuf, ext: &str, deps: &mut Deps) -> std::io::Result<()> {
        let output = self.on_file(path, ext, &mut deps.output)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        deps.missing = find(&stdout);
        if !output.status.success() {
            println!("Failed to build {}", path.display());
            std::io::stdout().write_all(&output.stdout)?;
            std::io::stdout().write_all(&output.stderr)?;
            Err(file_error("Failed to make"))
        } else {
            Ok(())
        }
    }
}

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
    //eprintln!("{:?}", options);
    let base = if options.dvi { "dvi" } else { "pdf" };

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

    let recipes = make_cmds();
    let mut deps = Deps::default();

    for file in options.files {
        let _ = recipes.get(base).unwrap().run_for(&file, base, &mut deps);
        let name = file
            .file_name()
            .unwrap()
            .to_str()
            .expect("Unsupported filename");
        collect_files(&name[..name.len() - ".tex".len()], &mut deps)?;

        let mut rerun = false;

        for dep in deps.input.iter() {
            if build(dep, &mut deps.output)? {
                rerun = true;
            }
        }
        for dep in deps.missing.iter() {
            if build(&PathBuf::default().with_file_name(&dep), &mut deps.output)? {
                rerun = true;
            }
        }

        if rerun {
            println!("Rerunning pdflatex");
            recipes.get(base).unwrap().run_for(&file, base, &mut deps)?;
        }
        deps.clean();
    }
    if options.clean {
        println!("Cleaning up files");
        for file in deps.output {
            let name = file.file_name().map_or("", |s| s.to_str().unwrap_or(""));
            // Protect pdf & dvi files
            if !name.ends_with("pdf") && !name.ends_with("dvi") {
                if let Err(_) = std::fs::remove_file(&file) {
                    if let Err(_) = std::fs::remove_dir_all(&file) {
                        println!("Couldn't remove {}", file.display());
                    }
                }
            }
        }
    }
    Ok(())
}

fn file_error(e: &'static str) -> Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}

fn collect_files(name: &str, deps: &mut Deps) -> std::io::Result<()> {
    let mut r = File::open(format!("./{}.fls", name))?;
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
            deps.input.insert(path);
        } else if cmd == "OUTPUT" {
            deps.output.insert(path);
        } else {
            panic!("Unexpected line: {}", cmd);
        }
    }
    Ok(())
}

fn build(dep: &PathBuf, output: &mut HashSet<PathBuf>) -> std::io::Result<bool> {
    let name = dep.file_name().map_or("", |o| o.to_str().unwrap_or(""));
    //println!("Building {}", name);
    let recipes = make_cmds();
    for (makes, recipe) in recipes.iter() {
        if name.ends_with(makes) {
            output.insert(dep.clone());
            let output = recipe.on_file(dep, makes, output)?;
            if output.status.success() {
                println!("Built {}", name);
                return Ok(true);
            } else {
                println!("Failed to build {}", name);
                std::io::stdout().write_all(&output.stdout)?;
                std::io::stdout().write_all(&output.stderr)?;
                return Ok(false);
            }
        }
    }
    Ok(false)
}
