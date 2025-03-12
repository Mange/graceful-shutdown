use crate::processes::Process;
use regex::RegexSet;

#[derive(Debug, Clone, Copy)]
pub enum MatchMode {
    Basename,
    Commandline,
}

#[derive(Debug)]
pub struct Matcher {
    regex_set: RegexSet,
    mode: MatchMode,
}

impl Matcher {
    pub fn new(regex_set: RegexSet, mode: MatchMode) -> Self {
        Matcher { regex_set, mode }
    }

    pub fn is_match(&self, process: &Process) -> bool {
        match self.mode {
            MatchMode::Basename => self.regex_set.is_match(process.name()),
            MatchMode::Commandline => self.regex_set.is_match(process.commandline()),
        }
    }
}
