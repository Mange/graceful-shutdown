extern crate users;

use crate::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use snafu::{ResultExt, Snafu, Whatever};
use std::fs::{self, DirEntry, ReadDir, read_dir, read_to_string};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use users::uid_t;

pub type ProcIter = Box<dyn Iterator<Item = Result<Process, Whatever>>>;

#[derive(Debug)]
pub struct Process {
    pid: Pid,
    user_id: uid_t,
    name: String,
    cmdline: String,
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
        .as_bytes()
        .iter()
        .all(|b| b.is_ascii_digit())
}

impl ProcessIterator {
    fn new() -> Result<ProcessIterator, Whatever> {
        Ok(ProcessIterator {
            read_dir: read_dir("/proc").whatever_context("Failed to read /proc")?,
        })
    }
}

impl Iterator for ProcessIterator {
    type Item = Result<Process, Whatever>;

    fn next(&mut self) -> Option<Self::Item> {
        // Read next dir entry. If it's not a directory, then skip to the next one again.
        // If entry failed to be loaded, skip that one too.
        match self.read_dir.next() {
            Some(Ok(entry)) => {
                if is_dir(&entry) && has_numeric_name(&entry) {
                    Some(Process::from_entry(&entry))
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
    type Item = Result<Process, Whatever>;

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
    pub fn all() -> Result<ProcIter, Whatever> {
        ProcessIterator::new().map(|iter| Box::new(iter) as ProcIter)
    }

    pub fn all_from_user(user: uid_t) -> Result<ProcIter, Whatever> {
        ProcessIterator::new().map(|iter| {
            Box::new(UserFilter {
                user,
                process_iter: iter,
            }) as ProcIter
        })
    }

    fn from_entry(entry: &DirEntry) -> Result<Process, Whatever> {
        let path = entry.path();
        let pid = {
            let basename = entry.file_name();
            let basename = basename.to_string_lossy();
            basename
                .parse()
                .with_whatever_context(|e| format!("Failed to parse PID in {}: {}", basename, e))?
        };

        let exe = fs::read_link(path.join("exe")).with_whatever_context(|e| {
            format!("Failed to determine executable of PID {pid}: {e}")
        })?;

        let name = exe.file_name().unwrap_or(exe.as_os_str()).to_string_lossy();
        let cmdline = stringify_cmdline(
            &read_to_string(path.join("cmdline"))
                .with_whatever_context(|_| format!("Failed to read {}", path.display()))?,
        );

        Ok(Process {
            name: name.into_owned(),
            cmdline,
            pid: Pid::from_raw(pid),
            user_id: uid_of_file(&path)?,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn commandline(&self) -> &str {
        &self.cmdline
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub fn is_alive(&self) -> bool {
        let mut proc_path = PathBuf::from("/proc");
        proc_path.push(self.pid.to_string());

        proc_path.exists()
    }

    pub fn send(&self, signal: Signal) -> Result<(), KillError> {
        use nix::errno::Errno;

        match kill(self.pid, signal) {
            Ok(()) => Ok(()),
            Err(Errno::EINVAL) => Err(KillError::InvalidSignal),
            Err(Errno::EPERM) => Err(KillError::NoPermission),
            Err(Errno::ESRCH) => Err(KillError::DoesNotExist),

            Err(errno) => Err(KillError::UnexpectedError {
                message: format!("errno {}", errno),
            }),
        }
    }
}

#[derive(Debug, Clone, Snafu)]
pub enum KillError {
    #[snafu(display("Invalid signal"))]
    InvalidSignal,
    #[snafu(display("Insufficient permission to send signal to this process"))]
    NoPermission,
    #[snafu(display("Cannot find process"))]
    DoesNotExist,
    #[snafu(display("Unexpected error: {message}"))]
    UnexpectedError { message: String },
}

fn uid_of_file(path: &Path) -> Result<uid_t, Whatever> {
    use std::os::linux::fs::MetadataExt;
    path.metadata()
        .with_whatever_context(|err| format!("Could not stat {}: {}", path.display(), err))
        .map(|metadata| metadata.st_uid())
}

fn stringify_cmdline(cmdline: &str) -> String {
    cmdline.replace("\0", " ").trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_cmdlines() {
        let input = "/usr/bin/bash\0-c\0echo hello world\0";
        let expected_output = "/usr/bin/bash -c echo hello world";

        assert_eq!(&stringify_cmdline(input), expected_output);
    }
}
