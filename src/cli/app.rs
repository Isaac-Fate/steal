use std::{ path::PathBuf, process::exit, time::Duration };
use clap::{ value_parser, Arg, Command };
use reqwest::Client;

use crate::{ Result, multipart::download };
use super::{
    greet::{ greet_command, handle_greet_command },
    info::{ handle_info_command, info_command },
};

pub async fn run_app() -> Result<()> {
    let command = create_command();
    let matches = command.get_matches();

    // Handle greet command
    handle_greet_command(&matches).await?;

    // Handle info command
    handle_info_command(&matches).await?;

    let url = matches.get_one::<String>("url").unwrap().to_owned();
    let dest_dir = if let Some(dest_dir) = matches.get_one::<PathBuf>("dest-dir") {
        dest_dir.to_owned()
    } else {
        std::env::current_dir()?
    };

    // Calculate the chunk size
    let kb = matches.get_one::<u64>("kb").unwrap_or(&0).to_owned();
    let mb = matches.get_one::<u64>("mb").unwrap_or(&0).to_owned();
    let segment_size = kb * 1024 + mb * 1024 * 1024;

    // Validate the chunk size
    if segment_size == 0 {
        println!("Segment size must be greater than 0");
        exit(1);
    }

    // Create an HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(60 * 60 * 24))
        .build()?;

    // Download!
    download(&client, &url, segment_size, &dest_dir).await?;

    Ok(())
}

fn create_command() -> Command {
    Command::new("Steal")
        .about("Download data from the internet quickly as if you were stealing from it 👻")
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

        // Greet command
        .subcommand(greet_command())

        // Command to get information of reponse headers
        .subcommand(info_command())
}
