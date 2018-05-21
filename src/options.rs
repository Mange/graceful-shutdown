extern crate structopt;
use structopt::clap::Shell;

extern crate users;
use users::uid_t;

use signal::Signal;
use std::time::Duration;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
/// Reads a list of commands to gracefully terminate from STDIN.
pub struct CliOptions {
    /// Number of seconds to wait for processes to terminate. Use 0 to disable waiting and exit
    /// immediately.
    #[structopt(short = "w", long = "wait-time", default_value = "5.0", value_name = "SECONDS")]
    wait_time: f64,

    /// Do not try to kill processes that do not exit within the waiting time, if a waiting time is
    /// set.
    #[structopt(long = "no-kill")]
    no_kill: bool,

    /// Signal to use when terminating processes.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[structopt(short = "s", long = "terminate-signal", default_value = "term",
                value_name = "SIGNAL", parse(try_from_str = "parse_signal"))]
    terminate_signal: Signal,

    /// Signal to use when killing processes that did not quit before the wait time ran out.
    ///
    /// Signals can be specified using signal number or symbolic name (case insensitive, with or
    /// without the SIG prefix).
    #[structopt(long = "kill-signal", default_value = "kill", value_name = "SIGNAL",
                parse(try_from_str = "parse_signal"))]
    kill_signal: Signal,

    /// Only find processes owned by the user with the given name.
    #[structopt(short = "u", long = "user", value_name = "USER")]
    user: Option<String>,

    /// Only find processes owned by you. Shortcut for --user "$USER". Has no effect if --user is
    /// specified.
    #[structopt(short = "m", long = "mine")]
    mine: bool,

    /// Don't actually send any signals to processes. Useful when testing configuration.
    #[structopt(short = "n", long = "dry-run")]
    dry_run: bool,

    /// Show more verbose output.
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,

    /// List all supported signals and exit.
    #[structopt(long = "list-signals")]
    pub list_signals: bool,

    /// Generate completion script for a given shell and output on STDOUT.
    #[structopt(long = "generate-completions", value_name = "SHELL",
                raw(possible_values = "&Shell::variants()"))]
    pub generate_completions: Option<Shell>,
}

#[derive(Debug)]
pub struct Options {
    pub wait_time: Option<Duration>,
    pub kill: bool,
    pub terminate_signal: Signal,
    pub kill_signal: Signal,
    pub user: Option<uid_t>,
    pub dry_run: bool,
    pub verbose: bool,
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

        Options {
            dry_run: cli_options.dry_run,
            kill: !cli_options.no_kill,
            kill_signal: cli_options.kill_signal,
            terminate_signal: cli_options.terminate_signal,
            user,
            verbose: cli_options.verbose,
            wait_time,
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
