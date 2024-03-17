use std::{ process::exit, time::Duration };
use clap::{ value_parser, Arg, ArgMatches, Command };
use indicatif::ProgressBar;

use crate::Result;

pub fn greet_command() -> Command {
    Command::new("greet")
        .about("Greet the user")
        .arg(
            Arg::new("number")
                .short('n')
                .long("number")
                .required(false)
                .value_parser(value_parser!(u64))
        )
}

pub async fn handle_greet_command(matches: &ArgMatches) -> Result<()> {
    if let Some(matches) = matches.subcommand_matches("greet") {
        if let Some(number) = matches.get_one::<u64>("number") {
            let number = number.to_owned();
            let progress_bar = ProgressBar::new(number);

            for _ in 0..100 {
                progress_bar.inc(1);
                std::thread::sleep(Duration::from_millis(10));
            }

            progress_bar.finish();
        }

        exit(0);
    }

    Ok(())
}
