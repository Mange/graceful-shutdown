extern crate libc;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Signal {
    SIGABRT,
    SIGALRM,
    SIGHUP,
    SIGINT,
    SIGKILL,
    SIGQUIT,
    SIGSTOP,
    SIGTERM,
    SIGUSR1,
    SIGUSR2,
}

const SIGNALS: [Signal; 10] = [
    Signal::SIGABRT,
    Signal::SIGALRM,
    Signal::SIGHUP,
    Signal::SIGINT,
    Signal::SIGKILL,
    Signal::SIGQUIT,
    Signal::SIGSTOP,
    Signal::SIGTERM,
    Signal::SIGUSR1,
    Signal::SIGUSR2,
];

pub struct SignalIterator {
    next: usize,
}

impl Signal {
    pub fn variants() -> SignalIterator {
        SignalIterator { next: 0 }
    }

    pub fn basename(self) -> &'static str {
        match self {
            Signal::SIGABRT => "SIGABRT",
            Signal::SIGALRM => "SIGALRM",
            Signal::SIGHUP => "SIGHUP",
            Signal::SIGINT => "SIGINT",
            Signal::SIGKILL => "SIGKILL",
            Signal::SIGQUIT => "SIGQUIT",
            Signal::SIGSTOP => "SIGSTOP",
            Signal::SIGTERM => "SIGTERM",
            Signal::SIGUSR1 => "SIGUSR1",
            Signal::SIGUSR2 => "SIGUSR2",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Signal::SIGABRT => "ABRT",
            Signal::SIGALRM => "ALRM",
            Signal::SIGHUP => "HUP",
            Signal::SIGINT => "INT",
            Signal::SIGKILL => "KILL",
            Signal::SIGQUIT => "QUIT",
            Signal::SIGSTOP => "STOP",
            Signal::SIGTERM => "TERM",
            Signal::SIGUSR1 => "USR1",
            Signal::SIGUSR2 => "USR2",
        }
    }

    pub fn number(self) -> i32 {
        match self {
            Signal::SIGABRT => libc::SIGABRT,
            Signal::SIGALRM => libc::SIGALRM,
            Signal::SIGHUP => libc::SIGHUP,
            Signal::SIGINT => libc::SIGINT,
            Signal::SIGKILL => libc::SIGKILL,
            Signal::SIGQUIT => libc::SIGQUIT,
            Signal::SIGSTOP => libc::SIGSTOP,
            Signal::SIGTERM => libc::SIGTERM,
            Signal::SIGUSR1 => libc::SIGUSR1,
            Signal::SIGUSR2 => libc::SIGUSR2,
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.name().fmt(f)
    }
}

impl Iterator for SignalIterator {
    type Item = Signal;

    fn next(&mut self) -> Option<Signal> {
        let res = SIGNALS.get(self.next).cloned();
        self.next += 1;
        res
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnknownSignalName,
}

impl FromStr for Signal {
    type Err = ParseError;

    fn from_str(sig: &str) -> Result<Signal, ParseError> {
        let upper_sig = {
            let mut s = String::from(sig);
            s.make_ascii_uppercase();
            s
        };

        let signal_number: Option<i32> = sig.parse().ok();

        for signal in Signal::variants() {
            if signal.basename() == upper_sig || signal.name() == upper_sig
                || signal_number
                    .map(|num| signal.number() == num)
                    .unwrap_or(false)
            {
                return Ok(signal);
            }
        }

        Err(ParseError::UnknownSignalName)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_strings_with_basename() {
        let sig: Signal = "kiLL".parse().expect("Failed to parse");
        assert_eq!(sig, Signal::SIGKILL);
    }

    #[test]
    fn it_parses_strings_with_name() {
        let sig: Signal = "SiGkiLL".parse().expect("Failed to parse");
        assert_eq!(sig, Signal::SIGKILL);
    }

    #[test]
    fn it_parses_strings_with_signal_number() {
        let string = Signal::SIGKILL.number().to_string();
        let sig: Signal = string.parse().expect("Failed to parse");
        assert_eq!(sig, Signal::SIGKILL);
    }

    #[test]
    fn it_does_not_parse_invalid_strings() {
        assert_eq!(
            "foobar".parse::<Signal>(),
            Err(ParseError::UnknownSignalName)
        );
        assert_eq!(
            "sigfoo".parse::<Signal>(),
            Err(ParseError::UnknownSignalName)
        );
        assert_eq!(
            "31337".parse::<Signal>(),
            Err(ParseError::UnknownSignalName)
        );
    }
}
