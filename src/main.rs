#[macro_use]
extern crate structopt;
use structopt::StructOpt;

extern crate regex;
use regex::{RegexSet, RegexSetBuilder};

extern crate atty;

extern crate users;

use std::io;
use std::io::BufRead;
use std::time::{Duration, Instant};

mod signal;
use signal::Signal;

mod processes;
use processes::Process;

mod options;
use options::{CliOptions, Options};

fn list_signals(verbose: bool) {
    if verbose {
        eprintln!("Currently supported signals:")
    };

    for signal in Signal::variants() {
        println!("{}\t{}", signal.number(), signal.name());
    }

    if verbose {
        eprintln!("Signal names does not require the SIG prefix, and are case-insensitive.");
    };
}

fn main() {
    let cli_options = CliOptions::from_args();

    if cli_options.list_signals {
        list_signals(cli_options.verbose);
        return;
    }

    let options = Options::from(cli_options);
    match run(options) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("ERROR: {}", err);
            ::std::process::exit(1);
        }
    }
}

fn run(options: Options) -> Result<(), String> {
    let matchers = load_patterns()?;

    let processes = all_processes(&options, &matchers)?;

    // Time to shut them down
    if options.dry_run {
        dry_run(&options, &processes)
    } else {
        real_run(&options, processes)
    }
}

fn load_patterns() -> Result<RegexSet, String> {
    if atty::is(atty::Stream::Stdin) {
        eprintln!("WARNING: Reading processlist from TTY stdin. Exit with ^D when you are done, or ^C to abort.");
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

fn all_processes(options: &Options, matchers: &RegexSet) -> Result<Vec<Process>, String> {
    let iter = match options.user {
        Some(ref uid) => Process::all_from_user(uid.to_owned())?,
        None => Process::all()?,
    };

    Ok(iter.flat_map(|result| match result {
        Ok(entry) => Some(entry),
        Err(_) => None,
    }).filter(|process| matchers.is_match(process.name()))
        .collect::<Vec<_>>())
}

fn strip_comment(line: String) -> String {
    match line.find("#") {
        Some(index) => line[0..index].trim().to_string(),
        None => line,
    }
}

fn dry_run(options: &Options, processes: &[Process]) -> Result<(), String> {
    for process in processes {
        println!(
            "Would have sent {} to process {} ({})",
            options.terminate_signal,
            process.pid(),
            process.name()
        );
    }

    Ok(())
}

fn real_run(options: &Options, mut processes: Vec<Process>) -> Result<(), String> {
    for process in &processes {
        if options.verbose {
            eprintln!(
                "Sending {} to process {} ({})",
                options.terminate_signal,
                process.pid(),
                process.name()
            );
        }
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

                if options.verbose && !is_alive {
                    eprintln!(
                        "Process {} ({}) has shut down",
                        process.pid(),
                        process.name()
                    );
                }

                is_alive
            });

            if processes.is_empty() {
                return Ok(());
            }
        }

        // Time is up. Kill remaining processes.
        if options.kill {
            if options.verbose {
                eprintln!("Timeout reached. Forcefully shutting down processes.");
            }
            for process in &processes {
                if options.verbose {
                    eprintln!(
                        "Sending {} to process {} ({})",
                        options.kill_signal,
                        process.pid(),
                        process.name()
                    );
                }
                process.send(options.kill_signal);
            }
        } else {
            eprintln!("WARNING: Some processes are still alive.");
            if options.verbose {
                for process in &processes {
                    eprintln!("Process {} ({})", process.pid(), process.name());
                }
            }
        }
    }

    Ok(())
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
