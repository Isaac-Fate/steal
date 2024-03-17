use std::process::exit;
use clap::{ value_parser, Arg, ArgMatches, Command };
use reqwest::Client;
use crate::{ Result, multipart::get_headers };

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

pub async fn handle_info_command(matches: &ArgMatches) -> Result<()> {
    if let Some(matches) = matches.subcommand_matches("info") {
        // Get URL
        let url = matches.get_one::<String>("url").unwrap().to_owned();

        // Create an HTTP client
        let client = Client::new();

        // Get headers
        let headers = get_headers(&client, &url).await?;

        // Print headers
        println!("{:#?}", headers);

        exit(0);
    }
    Ok(())
}
