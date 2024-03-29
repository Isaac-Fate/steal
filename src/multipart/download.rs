use std::{ path::Path, sync::Arc };
use indicatif::{ ProgressBar, ProgressStyle };
use reqwest::Client;
use tokio::{ fs::File, sync::Mutex };

use crate::{ Result, header::get_header_info };
use super::download_segment::download_segment;

pub async fn download<P: AsRef<Path>>(
    client: &Client,
    url: &str,
    segment_size: Option<u64>,
    num_threads: usize,
    dest_dir: P
) -> Result<()> {
    // Wrap client in an Arc
    let client = Arc::new(client.to_owned());

    // Get the response heaer info
    let header_info = get_header_info(&client, &url).await?;

    // Get the file size
    let file_size = header_info.content_length.unwrap();

    // Create a file
    let file_name = url.split('/').last().unwrap();
    let file_path = dest_dir.as_ref().join(file_name);
    let file = File::create(&file_path).await?;

    // Wrap file in an Arc
    let file = Arc::new(Mutex::new(file));

    // Create a progress bar
    let progress_bar = ProgressBar::new(file_size);

    // Style the progress bar
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})"
            )
            .unwrap()
            .progress_chars("#>-")
    );

    // Determine the segment size
    // If the segment size is not specified, use the file size divided by the number of threads
    let segment_size = match segment_size {
        Some(segment_size) => segment_size,
        None => file_size / (num_threads as u64),
    };

    // Split the data into multiple segments
    let ranges = split_data(file_size, segment_size);

    // Create async tasks
    let mut tasks: Vec<_> = ranges
        .into_iter()
        .map(|(start, end)| {
            // Clone the client
            let client = Arc::clone(&client);

            // Clone URL
            let url = url.to_owned();

            // Clone the file
            let file = Arc::clone(&file);

            // Clone the progress bar
            //
            // The progress bar is wrapped in an Arc so that it can be shared between multiple tasks
            //
            // https://arc.net/l/quote/scymebdb
            // The progress bar is an Arc around its internal state.
            // When the progress bar is cloned it just increments the refcount (so the original and its clone share the same state).
            //
            let progress_bar = progress_bar.clone();

            tokio::spawn(async move {
                download_segment(client, &url, start, end, file, &progress_bar).await.unwrap();
            })
        })
        .collect();

    // Wait for all tasks to complete
    while let Some(task) = tasks.pop() {
        task.await?;
    }

    // Prompt that the download is complete
    progress_bar.finish();
    println!("Saved to {}", file_path.display());

    Ok(())
}

fn split_data(file_size: u64, segment_size: u64) -> Vec<(u64, u64)> {
    // Initialize the ranges
    let mut ranges: Vec<(u64, u64)> = vec![];

    // Initialize the start
    let mut start = 0;

    while start < file_size {
        // Get the end
        let mut end = start + segment_size;

        // If the end is greater than the file size,
        // set it to the file size
        if end > file_size {
            end = file_size;
        }

        // Add the range
        // Note that we use `end - 1` here because
        // in HTTP range requests, the range (a, b) is inclusive,
        // which means the indices of bytes we request are a, a+1, a+2, ..., b
        ranges.push((start, end - 1));

        // Update the start
        start = end;
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download() -> Result<()> {
        // Create a client
        let client = reqwest::Client::new();

        // Resource URL
        let url = "http://ipv4.download.thinkbroadband.com/200MB.zip";

        // Destination directory
        let dest_dir = tempfile::tempdir()?;

        // Download!
        download(&client, url, Some(1024 * 1024 * 50), num_cpus::get(), dest_dir.path()).await?;

        Ok(())
    }
}
