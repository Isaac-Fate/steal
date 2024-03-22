use std::{ path::PathBuf, time::Duration };
use clap::{ value_parser, Arg, Command };
use reqwest::Client;

use crate::{ Result, multipart::download };
use super::{
    debug::{ debug_command, handle_debug_command },
    info::{ handle_info_command, info_command },
};

pub fn run_app() -> Result<()> {
    // Main app
    let app = app();

    // Get matches
    let matches: clap::ArgMatches = app.get_matches();

    // Handle debug command
    handle_debug_command(&matches)?;

    // Handle info command
    handle_info_command(&matches)?;

    // Resource URL
    let url = matches.get_one::<String>("url").unwrap().to_owned();

    // Destination directory
    let dest_dir = if let Some(dest_dir) = matches.get_one::<PathBuf>("dest-dir") {
        dest_dir.to_owned()
    } else {
        std::env::current_dir()?
    };

    // Number of threads
    let num_threads = matches
        .get_one::<usize>("num-threads")
        .unwrap_or(&num_cpus::get())
        .to_owned();

    // Calculate the chunk size
    let kb = matches.get_one::<u64>("kb").unwrap_or(&0).to_owned();
    let mb = matches.get_one::<u64>("mb").unwrap_or(&0).to_owned();
    let segment_size = kb * 1024 + mb * 1024 * 1024;

    // Validate the segment size
    let segment_size = if segment_size == 0 { None } else { Some(segment_size) };

    // Create a tokio runtime
    let runtime = tokio::runtime::Builder
        ::new_multi_thread()
        .worker_threads(num_threads)
        .enable_all()
        .build()?;

    // Create an HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(60 * 60 * 24))
        .build()?;

    // Download!
    runtime.block_on(async {
        println!("Using {} threads", num_threads);
        download(&client, &url, segment_size, num_threads, dest_dir).await
    })?;

    Ok(())
}

fn app() -> Command {
    Command::new("Steal")
        .about("Download data from the internet quickly as if you were stealing from it 👻")

        // Display number of CPUs available on the machine
        .after_help(format!("Number of CPUs: {}", num_cpus::get()))

        // If the user uses a subcommand,
        // then the args required by the main app are negated,
        // which is what I want!
        .subcommand_negates_reqs(true)

        // App args
        .arg(
            Arg::new("url")
                .required(true)
                .value_name("URL")
                .value_parser(value_parser!(String))
                .help("Resource URL")
        )
        .arg(
            Arg::new("dest-dir")
                .short('d')
                .long("dest-dir")
                .required(false)
                .value_name("DEST DIR")
                .value_parser(value_parser!(PathBuf))
                .help("Destination directory, defaults to current directory")
        )
        .arg(
            Arg::new("num-threads")
                .short('t')
                .long("threads")
                .required(false)
                .value_name("NUM THREADS")
                .value_parser(value_parser!(usize))
                .help("Number of threads to use for downloading")
        )
        .arg(
            Arg::new("kb")
                .short('k')
                .long("kb")
                .required(false)
                .value_name("KB")
                .value_parser(value_parser!(u64))
                .help("Part of each segment size in KB")
        )
        .arg(
            Arg::new("mb")
                .short('m')
                .long("mb")
                .required(false)
                .value_name("MB")
                .value_parser(value_parser!(u64))
                .help("Part of each segment size in MB")
        )

        // Debug command
        .subcommand(debug_command())

        // Command to get information of reponse headers
        .subcommand(info_command())
}
