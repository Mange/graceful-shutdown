#[macro_use]
extern crate structopt;
extern crate regex;
extern crate termion;
extern crate users;

mod matcher;
mod options;
mod processes;
mod signal;

use matcher::Matcher;
use regex::{RegexSet, RegexSetBuilder};
use signal::Signal;
use std::io;
use std::io::BufRead;
use std::time::{Duration, Instant};
use structopt::StructOpt;

use processes::Process;

use options::{CliOptions, Options};

fn list_signals() {
    // Print user-centric text if stdout is to a terminal. If piping stdout to some other process,
    // this text will not be shown.
    let is_tty = termion::is_tty(&::std::io::stdout());

    if is_tty {
        println!("Currently supported signals:")
    };

    for signal in Signal::variants() {
        println!("{}\t{}", signal.number(), signal.name());
    }

    if is_tty {
        println!("Signal names does not require the SIG prefix, and are case-insensitive.");
    };
}

fn generate_completions(shell: structopt::clap::Shell) {
    let mut app = CliOptions::clap();
    let name = app.get_name().to_string();

    app.gen_completions_to(name, shell, &mut io::stdout());
}

fn main() {
    use std::process::exit;
    let cli_options = CliOptions::from_args();

    if cli_options.list_signals {
        list_signals();
        return;
    }

    if let Some(shell) = cli_options.generate_completions {
        generate_completions(shell);
        return;
    }

    let options = Options::from(cli_options);
    match run(&options) {
        Ok(success) => if success {
            exit(0)
        } else {
            exit(1)
        },
        Err(err) => {
            if options.output_mode.show_normal() {
                eprintln!(
                    "{red}ERROR: {message}{reset}",
                    message = err,
                    red = options.colors.red(),
                    reset = options.colors.reset()
                );
            }
            exit(1);
        }
    }
}

fn run(options: &Options) -> Result<bool, String> {
    let matcher = Matcher::new(load_patterns(options)?, options.match_mode);

    let processes = all_processes(options, &matcher)?;

    // Time to shut them down
    if options.dry_run {
        dry_run(options, &processes)
    } else {
        real_run(options, processes)
    }
}

fn load_patterns(options: &Options) -> Result<RegexSet, String> {
    if options.output_mode.show_normal() && termion::is_tty(&::std::io::stdin()) {
        eprintln!(
            "{yellow}WARNING: Reading processlist from TTY stdin. Exit with ^D when you are done, or ^C to abort.{reset}",
            yellow = options.colors.yellow(),
            reset = options.colors.reset(),
        );
    }

    let stdin = io::stdin();
    let patterns: Vec<String> = stdin
        .lock()
        .lines()
        .flat_map(Result::ok)
        .map(strip_comment)
        .filter(|s| !s.is_empty())
        .collect();

    RegexSetBuilder::new(&patterns)
        .case_insensitive(true)
        .build()
        .map_err(|err| err.to_string())
}

fn all_processes(options: &Options, matcher: &Matcher) -> Result<Vec<Process>, String> {
    let iter = match options.user {
        Some(ref uid) => Process::all_from_user(uid.to_owned())?,
        None => Process::all()?,
    };

    Ok(iter
        .flat_map(Result::ok)
        .filter(|process| matcher.is_match(process))
        .collect::<Vec<_>>())
}

fn strip_comment(line: String) -> String {
    match line.find('#') {
        Some(index) => line[0..index].trim().to_string(),
        None => line,
    }
}

fn dry_run(options: &Options, processes: &[Process]) -> Result<bool, String> {
    // If we're not rendering anything, might as well skip the iteration completely.
    if !options.output_mode.show_normal() {
        return Ok(true);
    }

    for process in processes {
        println!(
            "Would have sent {signal} to process {process}",
            signal = options.terminate_signal,
            process = human_process_description(options, process),
        );
    }

    Ok(true)
}

fn real_run(options: &Options, mut processes: Vec<Process>) -> Result<bool, String> {
    for process in &processes {
        verbose_signal_message(options.terminate_signal, options, process);
        process.send(options.terminate_signal);
    }

    // Wait for processess to die
    if let Some(wait_time) = options.wait_time {
        let start = Instant::now();

        while start.elapsed() < wait_time {
            ::std::thread::sleep(Duration::from_millis(100));

            // Remove dead processes
            processes.retain(|process| {
                let is_alive = process.is_alive();

                if options.output_mode.show_verbose() && !is_alive {
                    eprintln!(
                        "Process shut down: {process}",
                        process = human_process_description(options, process),
                    );
                }

                is_alive
            });

            if processes.is_empty() {
                return Ok(true);
            }
        }

        // Time is up. Kill remaining processes.
        if options.kill {
            if options.output_mode.show_verbose() {
                eprintln!(
                    "{red}Timeout reached. Forcefully shutting down processes.{reset}",
                    red = options.colors.red(),
                    reset = options.colors.reset()
                );
            }
            for process in &processes {
                verbose_signal_message(options.kill_signal, options, process);
                process.send(options.kill_signal);
            }
        } else {
            if options.output_mode.show_normal() {
                eprintln!(
                    "{yellow}WARNING: Some processes are still alive.{reset}",
                    yellow = options.colors.yellow(),
                    reset = options.colors.reset()
                );
            }
            if options.output_mode.show_verbose() {
                for process in &processes {
                    eprintln!(
                        "Process {process}",
                        process = human_process_description(options, process)
                    );
                }
            }
            return Ok(false);
        }
    }

    Ok(true)
}

fn verbose_signal_message(signal: Signal, options: &Options, process: &Process) {
    if options.output_mode.show_verbose() {
        eprintln!(
            "Sending {signal} to process {process}",
            signal = signal,
            process = human_process_description(options, process),
        );
    }
}

fn human_process_description(options: &Options, process: &Process) -> String {
    use matcher::MatchMode;

    match options.match_mode {
        MatchMode::Basename => format!(
            "{green}{pid}{reset} ({green}{name}{reset})",
            pid = process.pid(),
            name = process.name(),
            green = options.colors.green(),
            reset = options.colors.reset()
        ),
        MatchMode::Commandline => format!(
            "{green}{pid}{reset} ({green}{name}{reset}): {faded}{cmdline}{reset}",
            pid = process.pid(),
            name = process.name(),
            cmdline = process.commandline(),
            green = options.colors.green(),
            faded = options.colors.faded(),
            reset = options.colors.reset(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_strips_comments() {
        assert_eq!(
            strip_comment(String::from("Foobar")),
            String::from("Foobar"),
        );

        assert_eq!(strip_comment(String::from("Foo#bar")), String::from("Foo"),);

        assert_eq!(
            strip_comment(String::from(" Complicated # oh yes!! # another one")),
            String::from("Complicated"),
        );

        assert_eq!(
            strip_comment(String::from("# Just a comment")),
            String::from(""),
        );

        assert_eq!(
            strip_comment(String::from("  \t# Just a comment")),
            String::from(""),
        );
    }
}
