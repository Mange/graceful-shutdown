extern crate termion;
extern crate users;

use crate::matcher::MatchMode;
use crate::signal::Signal;
use clap::Parser;
use clap_complete::Shell;
use std::{borrow::Cow, time::Duration};

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Normal,
    Verbose,
    Quiet,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Parser, Debug)]
/// Reads a list of commands to gracefully terminate from STDIN.
pub struct CliOptions {
    /// Number of seconds to wait for processes to terminate. Use 0 to disable waiting and exit
    /// immediately with a success status code.
    #[arg(
        short = 'w',
        long = "wait-time",
        default_value = "5.0",
        value_name = "SECONDS"
    )]
    wait_time: f64,

    /// Do not try to kill processes that do not exit within the waiting time, if a waiting time is
    /// set. Exits with an error status code if any matched process was still alive when waiting
    /// time is up.
    #[arg(long = "no-kill")]
    no_kill: bool,

    /// Signal to use when terminating processes.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[arg(
        short = 's',
        long = "terminate-signal",
        default_value = "term",
        value_name = "SIGNAL"
    )]
    terminate_signal: Signal,

    /// Signal to use when killing processes that did not quit before the wait time ran out.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[arg(long = "kill-signal", default_value = "kill", value_name = "SIGNAL")]
    kill_signal: Signal,

    /// Match the whole commandline for the process rather than the basename.
    #[arg(short = 'W', long = "whole-command", visible_alias = "whole")]
    match_whole: bool,

    /// Only find processes owned by the user with the given name.
    #[arg(
        short = 'u',
        long = "user",
        value_name = "USER",
        overrides_with = "mine"
    )]
    user: Option<String>,

    /// Only find processes owned by you. Shortcut for --user "$USER". Has no effect if --user is
    /// specified.
    #[arg(short = 'm', long = "mine", overrides_with = "user")]
    mine: bool,

    /// Don't actually send any signals to processes, instead show what actions would take place.
    /// Useful when testing configuration. This implies --verbose.
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,

    /// Show more verbose output.
    #[arg(short = 'v', long = "verbose", overrides_with = "quiet")]
    verbose: bool,

    /// Don't render any output.
    #[arg(short = 'q', long = "quiet", overrides_with = "verbose")]
    quiet: bool,

    /// Show color in command output. "auto" will enable color if output is sent to a TTY.
    #[arg(long = "color", default_value = "auto")]
    color_mode: ColorMode,

    /// List all supported signals and exit.
    #[arg(long = "list-signals")]
    pub list_signals: bool,

    /// Generate completion script for a given shell and output on STDOUT.
    #[arg(long = "generate-completions", value_name = "SHELL")]
    pub generate_completions: Option<Shell>,
}

#[derive(Debug)]
pub struct Options {
    pub dry_run: bool,
    pub kill: bool,
    pub kill_signal: Signal,
    pub match_mode: MatchMode,
    pub output_mode: OutputMode,
    pub terminate_signal: Signal,
    pub colors: Colors,
    pub user_mode: UserMode,
    pub wait_time: Option<Duration>,
}

#[derive(Debug)]
pub enum UserMode {
    Everybody,
    OnlyMe,
    Only(String),
}

#[derive(Debug)]
pub struct Colors {
    enabled: bool,
}

impl From<CliOptions> for Options {
    fn from(cli_options: CliOptions) -> Options {
        let wait_time = if cli_options.wait_time > 0.0 {
            Some(duration_from_secs_float(cli_options.wait_time))
        } else {
            None
        };

        let user_mode = match (cli_options.user, cli_options.mine) {
            (Some(name), false) => UserMode::Only(name),
            (None, true) => UserMode::OnlyMe,
            (None, false) => UserMode::Everybody,
            (Some(_), true) => unreachable!("Should not happen because of overrides_with"),
        };

        let match_mode = if cli_options.match_whole {
            MatchMode::Commandline
        } else {
            MatchMode::Basename
        };

        let output_mode = match (cli_options.dry_run, cli_options.verbose, cli_options.quiet) {
            // dry-run implies --verbose. Ignore the --quiet and --verbose flags!
            (true, _, _) => OutputMode::Verbose,

            // If not dry-run, then check the other flags.
            (false, false, false) => OutputMode::Normal,
            (false, true, false) => OutputMode::Verbose,
            (false, false, true) => OutputMode::Quiet,

            // Should never happen!
            (false, true, true) => unreachable!("Should not happen due to overrides_with option"),
        };

        let use_color = match cli_options.color_mode {
            ColorMode::Never => false,
            ColorMode::Always => true,
            ColorMode::Auto => termion::is_tty(&::std::io::stdout()),
        };

        Options {
            dry_run: cli_options.dry_run,
            kill: !cli_options.no_kill,
            kill_signal: cli_options.kill_signal,
            match_mode,
            output_mode,
            terminate_signal: cli_options.terminate_signal,
            colors: Colors { enabled: use_color },
            user_mode,
            wait_time,
        }
    }
}

impl OutputMode {
    pub fn is_normal(self) -> bool {
        match self {
            OutputMode::Verbose | OutputMode::Normal => true,
            OutputMode::Quiet => false,
        }
    }

    pub fn is_verbose(self) -> bool {
        match self {
            OutputMode::Verbose => true,
            OutputMode::Normal | OutputMode::Quiet => false,
        }
    }
}

impl Colors {
    pub fn reset(&self) -> Cow<str> {
        if self.enabled {
            Cow::Owned(format!(
                "{}{}",
                termion::color::Fg(termion::color::Reset),
                termion::style::Reset,
            ))
        } else {
            Cow::Borrowed("")
        }
    }

    pub fn red(&self) -> Cow<str> {
        if self.enabled {
            Cow::Owned(termion::color::Fg(termion::color::Red).to_string())
        } else {
            Cow::Borrowed("")
        }
    }

    pub fn yellow(&self) -> Cow<str> {
        if self.enabled {
            Cow::Owned(termion::color::Fg(termion::color::Yellow).to_string())
        } else {
            Cow::Borrowed("")
        }
    }

    pub fn green(&self) -> Cow<str> {
        if self.enabled {
            Cow::Owned(termion::color::Fg(termion::color::Green).to_string())
        } else {
            Cow::Borrowed("")
        }
    }

    pub fn faded(&self) -> Cow<str> {
        if self.enabled {
            Cow::Owned(termion::style::Faint.to_string())
        } else {
            Cow::Borrowed("")
        }
    }
}

fn duration_from_secs_float(float: f64) -> Duration {
    let whole_seconds = float.floor();
    let sec_frac = float - whole_seconds;
    let nanos = (sec_frac * 1e9).round();
    Duration::new(whole_seconds as u64, nanos as u32)
}
