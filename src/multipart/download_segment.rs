use std::{ io::SeekFrom, sync::Arc };
use indicatif::ProgressBar;
use reqwest::Client;
use tokio::{ fs::File, io::{ AsyncSeekExt, AsyncWriteExt }, sync::Mutex };
use futures::{ Stream, StreamExt };
use bytes::Bytes;
use reqwest::header::{ HeaderMap, HeaderValue, USER_AGENT, RANGE };

use crate::{ Result, Error };
use super::constants::USER_AGENT_VALUE;

pub async fn download_segment(
    client: Arc<Client>,
    url: &str,
    start: u64,
    end: u64,
    file: Arc<Mutex<File>>,
    progress_bar: Arc<Mutex<ProgressBar>>
) -> Result<()> {
    // Get data bytes stream
    let mut bytes_stream = get_segment_data_stream(client, url, start, end).await?;

    // Keep track of the file offset
    let mut file_offset = start;

    // Write each chunk in the stream to the file
    while let Some(bytes) = bytes_stream.next().await {
        // Unwrap the bytes
        let bytes = bytes?;

        // Aquire the lock
        let mut file = file.lock().await;

        // Seek to the right position
        file.seek(SeekFrom::Start(file_offset)).await?;

        // Write the bytes to the file
        file.write_all(&bytes).await?;

        // Get the chunk size
        let chunk_size = bytes.len() as u64;

        // Update the file offset
        file_offset += chunk_size;

        // Update the progress bar
        progress_bar.lock().await.inc(chunk_size);
    }

    Ok(())
}

async fn get_segment_data_stream(
    client: Arc<Client>,
    url: &str,
    start: u64,
    end: u64
) -> Result<impl Stream<Item = reqwest::Result<Bytes>>> {
    // Prepare the headers
    let mut headers = HeaderMap::new();

    // Set the user agent
    headers.insert(USER_AGENT, HeaderValue::from_str(USER_AGENT_VALUE).unwrap());

    // Set the range
    headers.insert(RANGE, format!("bytes={}-{}", start, end).parse().unwrap());

    // Send the request
    match client.get(url).headers(headers).send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Return the downloaded bytes stream
                Ok(response.bytes_stream())
            } else {
                Err(Error::ReqwestError(response.error_for_status().unwrap_err()))
            }
        }

        Err(err) => Err(Error::ReqwestError(err)),
    }
}
