use nix::sys::signal::Signal as NixSignal;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Signal(NixSignal);

impl Signal {
    pub fn iterator() -> impl Iterator<Item = Signal> {
        NixSignal::iterator().map(Signal)
    }

    pub fn name(self) -> String {
        format!("SIG{}", self.basename())
    }

    pub fn basename(self) -> &'static str {
        match self.0 {
            NixSignal::SIGABRT => "ABRT",
            NixSignal::SIGALRM => "ALRM",
            NixSignal::SIGHUP => "HUP",
            NixSignal::SIGINT => "INT",
            NixSignal::SIGKILL => "KILL",
            NixSignal::SIGQUIT => "QUIT",
            NixSignal::SIGSTOP => "STOP",
            NixSignal::SIGTERM => "TERM",
            NixSignal::SIGUSR1 => "USR1",
            NixSignal::SIGUSR2 => "USR2",
            NixSignal::SIGILL => "ILL",
            NixSignal::SIGTRAP => "TRAP",
            NixSignal::SIGBUS => "BUS",
            NixSignal::SIGFPE => "FPE",
            NixSignal::SIGSEGV => "SEGV",
            NixSignal::SIGPIPE => "PIPE",
            NixSignal::SIGSTKFLT => "STKFLT",
            NixSignal::SIGCHLD => "CHLD",
            NixSignal::SIGCONT => "CONT",
            NixSignal::SIGTSTP => "TSTP",
            NixSignal::SIGTTIN => "TTIN",
            NixSignal::SIGTTOU => "TTOU",
            NixSignal::SIGURG => "URG",
            NixSignal::SIGXCPU => "XCPU",
            NixSignal::SIGXFSZ => "XFSZ",
            NixSignal::SIGVTALRM => "VTALRM",
            NixSignal::SIGPROF => "PROF",
            NixSignal::SIGWINCH => "WINCH",
            NixSignal::SIGIO => "IO",
            NixSignal::SIGPWR => "PWR",
            NixSignal::SIGSYS => "SYS",
        }
    }

    pub fn number(self) -> i32 {
        self.0 as i32
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.basename().fmt(f)
    }
}

impl From<Signal> for NixSignal {
    fn from(signal: Signal) -> NixSignal {
        signal.0
    }
}

impl From<Signal> for Option<NixSignal> {
    fn from(signal: Signal) -> Option<NixSignal> {
        Some(signal.0)
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

        for signal in Signal::iterator() {
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
        assert_eq!(sig, Signal(NixSignal::SIGKILL));
    }

    #[test]
    fn it_parses_strings_with_name() {
        let sig: Signal = "SiGkiLL".parse().expect("Failed to parse");
        assert_eq!(sig, Signal(NixSignal::SIGKILL));
    }

    #[test]
    fn it_parses_strings_with_signal_number() {
        let string = Signal(NixSignal::SIGKILL).number().to_string();
        let sig: Signal = string.parse().expect("Failed to parse");
        assert_eq!(sig, Signal(NixSignal::SIGKILL));
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

    #[test]
    fn it_roundtrips_all_signals_parsing() {
        for signal in Signal::iterator() {
            assert_eq!(signal.basename().parse(), Ok(signal));
            assert_eq!(signal.name().parse(), Ok(signal));
            assert_eq!(signal.number().to_string().parse(), Ok(signal));
        }
    }
}
