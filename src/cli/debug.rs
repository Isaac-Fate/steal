use std::{ process::exit, time::Duration };
use clap::{ value_parser, Arg, ArgMatches, Command };
use indicatif::ProgressBar;

use crate::Result;

pub fn debug_command() -> Command {
    Command::new("debug")
        .about("For the developer to debug and play around")
        .arg(
            Arg::new("number")
                .short('n')
                .long("number")
                .required(false)
                .value_parser(value_parser!(u64))
        )

        // Hide this command since this is for debugging and playing around
        .hide(true)
}

pub fn handle_debug_command(matches: &ArgMatches) -> Result<()> {
    if let Some(matches) = matches.subcommand_matches("debug") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokio_runtime() -> Result<()> {
        let num_threads = num_cpus::get();
        println!("default numnber of threads: {}", num_threads);

        let runtime = tokio::runtime::Builder
            ::new_multi_thread()
            .worker_threads(16)
            .enable_all()
            .build()?;

        let task = runtime.spawn(async {
            echo("Hello, world!").await;
        });

        runtime.block_on(async {
            task.await.unwrap();
        });

        Ok(())
    }

    async fn echo(message: &str) {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        println!("{}", message);
    }
}
