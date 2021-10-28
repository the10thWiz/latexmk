//
// recipe.rs
// Copyright (C) 2021 matthew <matthew@matthew-ubuntu>
// Distributed under terms of the MIT license.
//

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, LinkedList},
    fs::File,
    io::{Error, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

use crate::{latex, sage, Options};

fn make_cmds(options: &Options) -> HashMap<String, Recipe> {
    let mut map = HashMap::new();
    latex::make_cmds(options, &mut map);
    sage::make_cmds(options, &mut map);
    // bibtex
    map.insert(
        "bbl".into(),
        Recipe {
            uses: "aux",
            f: &|_, _, _| Ok(()),
            extras: &["bib"],
            generated: &["blg"],
            generated_dirs: &[],
            script: "bibtex \"%N\"".into(),
        },
    );
    // use make
    map
}

/// Dependencies
#[derive(Debug, Default)]
pub struct Deps {
    /// Files that are read from
    input: HashSet<PathBuf>,
    /// Files that are output to
    output: HashSet<PathBuf>,
    /// Files reported as missing
    missing: HashSet<String>,
}

impl Deps {
    /// Clear the input and missing file lists
    fn clear(&mut self) {
        self.input.clear();
        self.missing.clear();
    }
}

/// Recipe struct
pub struct Recipe {
    /// The input file extension
    pub uses: &'static str,
    /// Function
    pub f: &'static dyn Fn(&PathBuf, &str, &mut Deps) -> std::io::Result<()>,
    /// Extra files used when running - Used when determining the file modification times
    pub extras: &'static [&'static str],
    /// Extra files generated - Used when determining the files to remove for clean operations
    pub generated: &'static [&'static str],
    /// Extra directories generated - Used when determining the files to remove for clean operations
    pub generated_dirs: &'static [&'static str],
    /// Command line string
    ///
    /// # Replacements
    /// - `%O`: The output file name
    /// - `%I`: The input file name
    /// - `%N`: The filename without an extension
    /// - `%%`: A literal percent
    pub script: Cow<'static, str>,
}

/// Calculates the parent of a given path
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
    /// Compare file modification times
    pub fn check_file_times(
        &self,
        input_name: &PathBuf,
        output_name: &str,
    ) -> std::io::Result<bool> {
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

    /// Run recipe for the provided path
    pub fn on_file(
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

    /// Run recipe for the provided path
    pub fn run_for(&self, path: &PathBuf, ext: &str, deps: &mut Deps) -> std::io::Result<()> {
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

/// Find `No file ` notes in output
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

/// Run commands to build recipe library, and run recipes as needed
pub fn run_cmds(mut options: Options) -> std::io::Result<()> {
    //eprintln!("{:?}", options);
    let base = if options.dvi { "dvi" } else { "pdf" };

    let recipes = make_cmds(&options);
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
            if build(dep, &mut deps.output, &recipes)? {
                rerun = true;
            }
        }
        for dep in deps.missing.iter() {
            if build(
                &PathBuf::default().with_file_name(&dep),
                &mut deps.output,
                &recipes,
            )? {
                rerun = true;
            }
        }

        if rerun {
            println!("Rerunning pdflatex");
            recipes.get(base).unwrap().run_for(&file, base, &mut deps)?;
        }
        deps.clear();
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

fn build(
    dep: &PathBuf,
    output: &mut HashSet<PathBuf>,
    recipes: &HashMap<String, Recipe>,
) -> std::io::Result<bool> {
    let name = dep.file_name().map_or("", |o| o.to_str().unwrap_or(""));
    //println!("Building {}", name);
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
