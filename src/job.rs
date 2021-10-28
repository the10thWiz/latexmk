//
// job.rs
// Copyright (C) 2021 matthew <matthew@WINDOWS-05HIC4F>
// Distributed under terms of the MIT license.
//

use std::{
    collections::{HashMap, HashSet, LinkedList},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    recipe::{recipes, Recipe},
    util::replace_file_ext,
    Options,
};

pub fn run(options: Options) -> std::io::Result<()> {
    let mut queue = JobQueue {
        jobs: LinkedList::new(),
        files: HashSet::new(),
        recipes: recipes(&options),
        texfile: PathBuf::from_str(".").unwrap(),
        rerun_current_job: false,
    };
    let output_ext = if options.dvi { "dvi" } else { "pdf" };

    for file in options.files {
        queue.insert(replace_file_ext(&file, "tex", output_ext), file);
        queue.execute()?;
    }
    Ok(())
}

pub struct JobQueue {
    jobs: LinkedList<Job>,
    files: HashSet<PathBuf>,
    recipes: HashMap<String, Recipe>,
    texfile: PathBuf,
    rerun_current_job: bool,
}

impl JobQueue {
    fn execute(&mut self) -> std::io::Result<()> {
        if let Some(job) = self.jobs.pop_front() {
            let _ = job.execute(self);
        }
        while let Some(job) = self.jobs.pop_front() {
            job.execute(self)?;
        }
        Ok(())
    }

    /// Register an output file or directory that has been generated
    ///
    /// Note that the file does not need to exist, so files that are only sometimes generated
    /// can be added reguardless of whether the file was actually generated
    pub fn output(&mut self, file: PathBuf) {
        self.files.insert(file);
    }

    pub fn tex_file(&self) -> &Path {
        &self.texfile
    }

    /// Marks that the current job requires a file to be built
    ///
    /// Note: this internally sets the rerun flag, so rerun should not be called unless there
    /// is a seperate reason to rerun the job. The rerun flag is ONLY set if the requested file
    /// is actually built.
    pub fn needs(&mut self, file: PathBuf) {
        let name = file.file_name().map_or("", |f| f.to_str().unwrap_or(""));
        for (ext, recipe) in self.recipes.iter() {
            if name.ends_with(ext) {
                let recipe = recipe.clone();
                if (recipe.needs_to_run)(&file, self) {
                    self.jobs.push_back(Job { recipe, on: file });
                    self.rerun_current_job = true;
                }
                break;
            }
        }
    }

    pub fn insert(&mut self, file: PathBuf, texfile: PathBuf) {
        self.texfile = texfile;
        let name = file.file_name().map_or("", |f| f.to_str().unwrap_or(""));
        for (ext, recipe) in self.recipes.iter() {
            if name.ends_with(ext) {
                self.jobs.push_back(Job {
                    recipe: recipe.clone(),
                    on: file,
                });
                self.rerun_current_job = true;
                break;
            }
        }
    }

    /// Marks the current job to be rerun.
    pub fn rerun(&mut self) {
        self.rerun_current_job = true;
    }

    fn register_job(&mut self, job: Job) {
        self.jobs.push_back(job);
    }
}

#[derive(Clone)]
pub struct Job {
    recipe: Recipe,
    on: PathBuf,
}

impl Job {
    fn execute(self, queue: &mut JobQueue) -> std::io::Result<()> {
        queue.rerun_current_job = false;
        let res = (self.recipe.run)(&self.on, queue);
        if queue.rerun_current_job {
            queue.register_job(self);
        }
        res
    }
}
