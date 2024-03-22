use std::process::exit;
use clap::{ value_parser, Arg, ArgMatches, Command };
use reqwest::Client;
use crate::{ Result, get_headers };

pub fn info_command() -> Command {
    Command::new("info")
        .about("Get the information of the response headers")
        .arg(
            Arg::new("url")
                .required(true)
                .value_name("URL")
                .value_parser(value_parser!(String))
                .help("Resource URL")
        )
}

pub fn handle_info_command(matches: &ArgMatches) -> Result<()> {
    // Create a tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(async {
        if let Some(matches) = matches.subcommand_matches("info") {
            // Get URL
            let url = matches.get_one::<String>("url").unwrap().to_owned();

            // Create an HTTP client
            let client = Client::new();

            // Get headers
            let headers = get_headers(&client, &url).await.unwrap();

            // Print headers
            println!("{:#?}", headers);

            exit(0);
        }
    });

    Ok(())
}
