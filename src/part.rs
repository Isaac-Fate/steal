use std::{ io::SeekFrom, sync::Arc };
use bytes::Bytes;
use futures::{ Stream, StreamExt };
use indicatif::ProgressBar;
use reqwest::{ header::{ HeaderMap, HeaderValue, RANGE, USER_AGENT }, Client };
use tokio::{ fs::File, io::{ AsyncSeekExt, AsyncWriteExt }, sync::Mutex };
use crate::{ header::{ get_headers, USER_AGENT_VALUE }, Error, Result };

async fn download(client: Client, url: &str, segment_size: u64, file: File) -> Result<()> {
    // Wrap client in an Arc
    let client = Arc::new(client);

    // Wrap file in an Arc
    let file = Arc::new(Mutex::new(file));

    // Get response headers
    let headers = get_headers(&client, url).await?;

    // Get the file size
    let file_size = headers
        .get("content-length")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();

    // Create a progress bar, and
    // wrap it in an Arc
    let progress_bar = ProgressBar::new(file_size);
    let progress_bar = Arc::new(Mutex::new(progress_bar));

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
            let progress_bar = Arc::clone(&progress_bar);

            tokio::spawn(async move {
                download_segment(client, &url, start, end, file, progress_bar).await.unwrap();
            })
        })
        .collect();

    // Wait for all tasks to complete
    while let Some(task) = tasks.pop() {
        task.await?;
    }

    Ok(())
}

async fn download_segment(
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

async fn get_segment_data(client: Arc<Client>, url: &str, start: u64, end: u64) -> Result<Bytes> {
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
                // Return the downloaded bytes
                Ok(response.bytes().await?)
            } else {
                Err(Error::ReqwestError(response.error_for_status().unwrap_err()))
            }
        }

        Err(err) => Err(Error::ReqwestError(err)),
    }
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

// fn write_segment_data_to_file(file: Arc<Mutex<File>>, start: u64, bytes: Bytes) -> Result<()> {

//     Ok(())
// }

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
    use std::sync::Arc;
    use futures::StreamExt;
    use indicatif::ProgressBar;
    use crate::header::get_headers;
    use super::*;

    #[tokio::test]
    async fn test_download() -> Result<()> {
        // Create a client
        let client = Client::new();

        // Resource URL
        let url = "http://ipv4.download.thinkbroadband.com/200MB.zip";

        // Destination file
        let file_name = url.split('/').last().unwrap();
        let file = File::create(file_name).await?;

        // Segment size
        let segment_size: u64 = 1024 * 1024 * 1;

        // Download!
        download(client, url, segment_size, file).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_download_segment() -> Result<()> {
        // Create a client
        let client = Client::new();

        // Wrap the client in an Arc
        let client = Arc::new(client);

        // Resource URL
        let url = "http://ipv4.download.thinkbroadband.com/200MB.zip";

        // Get response headers
        let headers = get_headers(&client, url).await?;

        // Print the headers
        println!("{:#?}", headers);

        // Get file size
        let file_size = headers
            .get("content-length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        // Split the data to download into multiple segments
        let ranges = split_data(file_size, 1024 * 1024);

        // Range
        let (start, end) = ranges[0];

        // Initialize the progress bar
        let progress_bar = Arc::new(Mutex::new(ProgressBar::new(end - start + 1)));

        // Get file name
        let file_name = url.split('/').last().unwrap();

        // Open the file
        let file = File::create(file_name).await?;

        // Wrap the file in an Arc
        let file = Arc::new(Mutex::new(file));

        download_segment(client, url, start, end, file, progress_bar).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_segment_data() -> Result<()> {
        // Create a client
        let client = Client::new();

        // Wrap the client in an Arc
        let client = Arc::new(client);

        // Resource URL
        let url = "http://ipv4.download.thinkbroadband.com/200MB.zip";

        // Get response headers
        let headers = get_headers(&client, url).await?;

        // Print the headers
        println!("{:#?}", headers);

        let ranges: Vec<(u64, u64)> = vec![(0, 31), (32, 63)];
        let mut tasks: Vec<_> = ranges
            .into_iter()
            .map(|(start, end)| {
                let client = Arc::clone(&client);
                tokio::spawn(async move { get_segment_data(client, url, start, end) })
            })
            .collect();

        for task in tasks.drain(..) {
            let bytes = task.await?.await?;
            println!("{:#?}", bytes);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_get_segment_data_stream() -> Result<()> {
        // Create a client
        let client = Client::new();

        // Wrap the client in an Arc
        let client = Arc::new(client);

        // Resource URL
        let url = "http://ipv4.download.thinkbroadband.com/200MB.zip";

        // Get response headers
        let headers = get_headers(&client, url).await?;

        // Print the headers
        println!("{:#?}", headers);

        // Get the file size
        let file_size: u64 = headers
            .get("content-length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();

        // Progress bar
        let progress_bar = ProgressBar::new(1024 * 1024);

        let ranges: Vec<(u64, u64)> = vec![(0, 1024 * 1024 - 1)];
        let mut tasks: Vec<_> = ranges
            .into_iter()
            .map(|(start, end)| {
                let client = Arc::clone(&client);
                tokio::spawn(async move { get_segment_data_stream(client, url, start, end) })
            })
            .collect();

        for task in tasks.drain(..) {
            let mut stream = task.await?.await?;

            while let Some(bytes) = stream.next().await {
                let bytes = bytes?;

                progress_bar.inc(bytes.len() as u64);
            }
        }

        Ok(())
    }
}
