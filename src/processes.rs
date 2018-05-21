extern crate libc;
extern crate users;
use users::uid_t;

use std::fs::{read_dir, DirEntry, File, ReadDir};
use std::path::{Path, PathBuf};
use std::io::Read;

use signal::Signal;

pub type ProcIter = Box<Iterator<Item = Result<Process, String>>>;

#[derive(Debug)]
pub struct Process {
    pid: i32,
    user_id: uid_t,
    name: String,
}

pub struct ProcessIterator {
    read_dir: ReadDir,
}

pub struct UserFilter {
    user: uid_t,
    process_iter: ProcessIterator,
}

fn is_dir(entry: &DirEntry) -> bool {
    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
}

fn has_numeric_name(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_string_lossy()
        .bytes()
        .all(|b| b >= b'0' && b <= b'9')
}

impl ProcessIterator {
    fn new() -> Result<ProcessIterator, String> {
        Ok(ProcessIterator {
            read_dir: read_dir("/proc").map_err(|err| format!("Failed to open /proc: {}", err))?,
        })
    }
}

impl Iterator for ProcessIterator {
    type Item = Result<Process, String>;

    fn next(&mut self) -> Option<Self::Item> {
        // Read next dir entry. If it's not a directory, then skip to the next one again.
        // If entry failed to be loaded, skip that one too.
        match self.read_dir.next() {
            Some(Ok(entry)) => {
                if is_dir(&entry) && has_numeric_name(&entry) {
                    Some(Process::from_entry(entry))
                } else {
                    self.next()
                }
            }
            Some(Err(_)) => self.next(),
            None => None,
        }
    }
}

impl Iterator for UserFilter {
    type Item = Result<Process, String>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.process_iter.next() {
            Some(Ok(process)) => {
                if process.user_id == self.user {
                    Some(Ok(process))
                } else {
                    self.next()
                }
            }
            other => other,
        }
    }
}

impl Process {
    pub fn all() -> Result<ProcIter, String> {
        ProcessIterator::new().map(|iter| Box::new(iter) as ProcIter)
    }

    pub fn all_from_user(user: uid_t) -> Result<ProcIter, String> {
        ProcessIterator::new().map(|iter| {
            Box::new(UserFilter {
                user: user,
                process_iter: iter,
            }) as ProcIter
        })
    }

    fn from_entry(entry: DirEntry) -> Result<Process, String> {
        let path = entry.path();
        let name = read_file(&path.join("comm"))?.trim_right().to_string();
        let pid = {
            let basename = entry.file_name();
            let basename = basename.to_string_lossy();
            basename
                .parse()
                .map_err(|e| format!("Failed to parse PID in {}: {}", basename, e))?
        };

        Ok(Process {
            name,
            pid,
            user_id: uid_of_file(&path)?,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn is_alive(&self) -> bool {
        let mut proc_path = PathBuf::new();
        proc_path.push("/");
        proc_path.push("proc");
        proc_path.push(self.pid.to_string());

        proc_path.exists()
    }

    pub fn send(&self, signal: Signal) {
        unsafe {
            if libc::kill(self.pid, signal.number()) < 0 {
                panic!("Call to kill failed.");
            }
        }
    }
}

fn read_file(path: &Path) -> Result<String, String> {
    // In Rust 1.26 we can use Path::read_to_string instead.
    let mut string = String::new();
    let mut file =
        File::open(path).map_err(|e| format!("Could not open file {}: {}", path.display(), e))?;
    file.read_to_string(&mut string)
        .map_err(|e| format!("Could not read file {}: {}", path.display(), e))?;
    Ok(string)
}

fn uid_of_file(path: &Path) -> Result<uid_t, String> {
    use std::os::linux::fs::MetadataExt;
    path.metadata()
        .map_err(|err| format!("Could not stat {}: {}", path.display(), err))
        .map(|metadata| metadata.st_uid())
}
