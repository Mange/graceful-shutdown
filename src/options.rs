extern crate structopt;
extern crate termion;
extern crate users;

use matcher::MatchMode;
use signal::Signal;
use std::time::Duration;
use structopt::clap::Shell;
use users::uid_t;

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Normal,
    Verbose,
    Quiet,
}

#[derive(Debug, Clone, Copy)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
/// Reads a list of commands to gracefully terminate from STDIN.
pub struct CliOptions {
    /// Number of seconds to wait for processes to terminate. Use 0 to disable waiting and exit
    /// immediately with a success status code.
    #[structopt(short = "w", long = "wait-time", default_value = "5.0", value_name = "SECONDS")]
    wait_time: f64,

    /// Do not try to kill processes that do not exit within the waiting time, if a waiting time is
    /// set. Exits with an error status code if any matched process was still alive when waiting
    /// time is up.
    #[structopt(long = "no-kill")]
    no_kill: bool,

    /// Signal to use when terminating processes.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[structopt(
        short = "s",
        long = "terminate-signal",
        default_value = "term",
        value_name = "SIGNAL",
        parse(try_from_str = "parse_signal")
    )]
    terminate_signal: Signal,

    /// Signal to use when killing processes that did not quit before the wait time ran out.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[structopt(
        long = "kill-signal",
        default_value = "kill",
        value_name = "SIGNAL",
        parse(try_from_str = "parse_signal")
    )]
    kill_signal: Signal,

    /// Match the whole commandline for the process rather than the basename.
    #[structopt(short = "W", long = "whole-command", visible_alias = "whole")]
    match_whole: bool,

    /// Only find processes owned by the user with the given name.
    #[structopt(short = "u", long = "user", value_name = "USER")]
    user: Option<String>,

    /// Only find processes owned by you. Shortcut for --user "$USER". Has no effect if --user is
    /// specified.
    #[structopt(short = "m", long = "mine")]
    mine: bool,

    /// Don't actually send any signals to processes, instead show what actions would take place.
    /// Useful when testing configuration. This implies --verbose.
    #[structopt(short = "n", long = "dry-run")]
    dry_run: bool,

    /// Show more verbose output.
    #[structopt(short = "v", long = "verbose", overrides_with = "quiet")]
    verbose: bool,

    /// Don't render any output.
    #[structopt(short = "q", long = "quiet", overrides_with = "verbose")]
    quiet: bool,

    /// Show color in command output. "auto" will enable color if output is sent to a TTY.
    #[structopt(
        long = "color", default_value = "auto", raw(possible_values = "&ColorMode::variants()")
    )]
    color_mode: ColorMode,

    /// List all supported signals and exit.
    #[structopt(long = "list-signals")]
    pub list_signals: bool,

    /// Generate completion script for a given shell and output on STDOUT.
    #[structopt(
        long = "generate-completions",
        value_name = "SHELL",
        raw(possible_values = "&Shell::variants()")
    )]
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
    pub user: Option<uid_t>,
    pub wait_time: Option<Duration>,
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

        let mine = cli_options.mine;
        let user = cli_options
            .user
            .map(|name| {
                users::get_user_by_name(&name)
                    .expect("Could not find user")
                    .uid()
            })
            .or_else(|| {
                if mine {
                    Some(users::get_current_uid())
                } else {
                    None
                }
            });

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
            user,
            wait_time,
        }
    }
}

impl OutputMode {
    pub fn show_normal(&self) -> bool {
        match self {
            OutputMode::Verbose | OutputMode::Normal => true,
            OutputMode::Quiet => false,
        }
    }

    pub fn show_verbose(&self) -> bool {
        match self {
            OutputMode::Verbose => true,
            OutputMode::Normal | OutputMode::Quiet => false,
        }
    }
}

impl ColorMode {
    fn variants() -> [&'static str; 3] {
        ["auto", "always", "never"]
    }
}

impl ::std::str::FromStr for ColorMode {
    type Err = &'static str;

    fn from_str(string: &str) -> Result<ColorMode, Self::Err> {
        match string {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err("Not a valid color mode"),
        }
    }
}

impl Colors {
    pub fn reset(&self) -> String {
        if self.enabled {
            format!(
                "{}{}",
                termion::color::Fg(termion::color::Reset),
                termion::style::Reset,
            )
        } else {
            String::new()
        }
    }

    pub fn red(&self) -> String {
        if self.enabled {
            termion::color::Fg(termion::color::Red).to_string()
        } else {
            String::new()
        }
    }

    pub fn yellow(&self) -> String {
        if self.enabled {
            termion::color::Fg(termion::color::Yellow).to_string()
        } else {
            String::new()
        }
    }

    pub fn green(&self) -> String {
        if self.enabled {
            termion::color::Fg(termion::color::Green).to_string()
        } else {
            String::new()
        }
    }

    pub fn faded(&self) -> String {
        if self.enabled {
            termion::style::Faint.to_string()
        } else {
            String::new()
        }
    }
}

fn parse_signal(sig: &str) -> Result<Signal, String> {
    sig.parse()
        .map_err(|_| format!("Failed to parse \"{}\" as a signal name.", sig))
}

fn duration_from_secs_float(float: f64) -> Duration {
    let whole_seconds = float.floor();
    let sec_frac = float - whole_seconds;
    let nanos = (sec_frac * 1e9).round();
    Duration::new(whole_seconds as u64, nanos as u32)
}
