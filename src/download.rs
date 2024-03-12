use std::{ fs::File, io::{ Read, Seek, SeekFrom, Write }, path::Path, sync::{ Arc, Mutex } };
use bytes::{ Buf, Bytes };
use reqwest::{ header::{ HeaderMap, HeaderValue, USER_AGENT, RANGE }, Client };
use crate::{ Error, Result };

const USER_AGENT_VALUE: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36 Edg/112.0.1722.48";

pub async fn download<P: AsRef<Path>>(
    client: &Client,
    url: &str,
    chunk_size: u64,
    dest_dir: P
) -> Result<()> {
    // Get the file name
    let file_name = url.split('/').last().unwrap();

    // Create the destination file path
    let file_path = dest_dir.as_ref().join(file_name);

    // Get the headers
    let headers = get_headers(client, url).await?;

    // Log response headers
    println!("response headers:\n{:#?}", headers);

    // Get the file size
    let file_size: u64 = headers.get("content-length").unwrap().to_str().unwrap().parse().unwrap();

    // Get the ranges
    let ranges = split_data(file_size, chunk_size);

    // Create a tmp dir
    let dir = tempfile::tempdir()?;

    // Log the tmp dir path
    println!("tmp dir created at: {}", dir.path().display());

    // Wrap client in an Arc
    let client = Arc::new(client.clone());

    // Create async tasks
    let mut tasks: Vec<_> = ranges
        .into_iter()
        .map(|(start, end)| {
            // Clone the client
            let client = Arc::clone(&client);

            // Clone URL
            let url = url.to_owned();

            // Get tmp dir path
            let dir = dir.path().to_path_buf();

            // Download each chunk of the data
            tokio::spawn(async move {
                download_chunk(client, &url, start, end, dir).await.unwrap();

                // Log
                println!("Downloaded chunk from {} to {}", start, end);
            })
        })
        .collect();

    // Wait for all tasks to complete
    for task in tasks.drain(..) {
        task.await?;
    }

    // Merge all tmp files of chunks
    let mut file = std::fs::OpenOptions::new().write(true).create(true).open(&file_path)?;

    // Get all chunk files
    let mut chunk_file_paths: Vec<_> = dir
        .path()
        .read_dir()?
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .collect();

    // Sort the chunk file paths
    chunk_file_paths.sort_by(|path1, path2| {
        let num1: u64 = path1.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let num2: u64 = path2.file_stem().unwrap().to_str().unwrap().parse().unwrap();

        num1.cmp(&num2)
    });

    for chunk_file_path in chunk_file_paths {
        // Open the file contating the chunk
        let mut chunk_file = std::fs::OpenOptions::new().read(true).open(&chunk_file_path)?;

        // Create a buffer to store the chunk content
        let mut chunk_buffer: Vec<u8> = vec![];

        // Read chunk content
        chunk_file.read_to_end(&mut chunk_buffer)?;

        // Write chunk content to the destination file
        file.write_all(&chunk_buffer)?;
    }

    Ok(())
}

pub async fn get_headers(client: &Client, url: &str) -> Result<HeaderMap> {
    // Prepare the headers
    let mut headers = HeaderMap::new();

    // Set the user agent
    headers.insert(USER_AGENT, HeaderValue::from_str(USER_AGENT_VALUE).unwrap());

    // Send the request
    match client.head(url).headers(headers).send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Return the response headers
                Ok(response.headers().clone())
            } else {
                Err(Error::ReqwestError(response.error_for_status().unwrap_err()))
            }
        }

        Err(err) => Err(Error::ReqwestError(err)),
    }
}

pub async fn download_chunk<P: AsRef<Path>>(
    client: Arc<Client>,
    url: &str,
    start: u64,
    end: u64,
    dir: P
) -> Result<()> {
    // Get the chunk data
    let bytes = get_chunk_data(client, url, start, end).await?;

    // Create file path
    let filepath = dir.as_ref().join(format!("{}", start));

    // Create a file
    let mut file = File::create(filepath)?;

    // Write the chunk to the file
    file.write_all(bytes.chunk())?;

    Ok(())
}

// pub async fn _download_chunk(
//     client: Arc<Client>,
//     url: &str,
//     start: u64,
//     end: u64,
//     file: &Arc<Mutex<File>>
// ) -> Result<()> {
//     // Get the chunk data
//     let bytes = get_chunk_data(client, url, start, end).await?;

//     // Write the chunk to the file
//     _write_chunk_to_file(file, start, bytes)
// }

async fn get_chunk_data(client: Arc<Client>, url: &str, start: u64, end: u64) -> Result<Bytes> {
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

fn _write_chunk_to_file(file: &Arc<Mutex<File>>, start: u64, bytes: Bytes) -> Result<()> {
    // Acquire the lock
    let mut file = file.lock().unwrap();

    // Move to the correct position
    file.seek(SeekFrom::Start(start))?;

    // Write the bytes
    file.write_all(bytes.chunk())?;

    Ok(())
}

fn split_data(file_size: u64, chunk_size: u64) -> Vec<(u64, u64)> {
    // Initialize the ranges
    let mut ranges: Vec<(u64, u64)> = vec![];

    // Initialize the start
    let mut start = 0;

    while start < file_size {
        // Get the end
        let mut end = start + chunk_size;

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
    use std::{
        io::{ Read, Seek, SeekFrom, Write },
        path::Path,
        sync::{ Arc, Mutex },
        time::Duration,
    };
    use bytes::Bytes;
    use reqwest::{ Client, header::{ HeaderMap, HeaderValue, RANGE } };
    use rayon::{ prelude::*, ThreadPoolBuilder };
    use crate::Result;
    use super::{ download, get_headers, get_chunk_data, _write_chunk_to_file, split_data };

    #[tokio::test]
    async fn test_download() -> Result<()> {
        // Create a client
        let client = Client::builder()
            .timeout(Duration::from_secs(60 * 60 * 24))
            .build()?;
        // let client = Arc::new(&client);

        // Download!
        download(
            &client,
            // "https://visiondata.cis.upenn.edu/volumetric/h36m/h36m_annot.tar",
            "https://thor.robots.ox.ac.uk/datasets/mjsynth/mjsynth.tar.gz",
            1024 * 1024 * 100,
            Path::new("/Users/isaac/Downloads")
        ).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_headers() -> Result<()> {
        // Create a client
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;

        // Get response headers
        let headers = get_headers(
            &client,
            "https://visiondata.cis.upenn.edu/volumetric/h36m/h36m_annot.tar"
        ).await?;

        println!("{:#?}", headers);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_chunk_data() -> Result<()> {
        // Create a client
        let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
        let client = Arc::new(client);

        let bytes = get_chunk_data(
            client,
            "https://visiondata.cis.upenn.edu/volumetric/h36m/h36m_annot.tar",
            0,
            15
        ).await?;

        println!("bytes length: {}", bytes.len());

        println!("{:#?}", String::from_utf8(bytes.to_vec()).unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_write_chunk_to_file() -> Result<()> {
        // Create a temp dir
        let dir = tempfile::tempdir()?;

        // let dir = Path::new("./");
        println!("tmp dir created at: {}", dir.path().display());

        // Create a file to write to
        let file = std::fs::OpenOptions
            ::new()
            .read(true)
            .write(true)
            .create(true)
            .open(dir.path().join("test.txt"))?;

        // Wrap the file in an Arc to allow it to be shared between threads
        let file = Arc::new(Mutex::new(file));

        // Bytes to write
        let bytes = Bytes::from("Hello World!");

        // Write to the file
        _write_chunk_to_file(&file, 0, bytes)?;

        // Read the file
        let mut file = file.lock().unwrap();
        file.seek(SeekFrom::Start(0))?;
        let mut content = String::new();
        let file_size = file.read_to_string(&mut content)?;

        // Print the content
        println!("file content:\n{}", content);
        assert_eq!(file_size, 12);
        assert_eq!(content, "Hello World!");

        // Close the dir to clean up
        dir.close()?;

        Ok(())
    }

    #[tokio::test]
    async fn get_data() {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build().unwrap();

        let mut headers = HeaderMap::new();

        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36 Edg/112.0.1722.48"
            ).unwrap()
        );
        headers.insert(RANGE, "bytes=0-31".parse().unwrap());

        let response = client
            .get("https://visiondata.cis.upenn.edu/volumetric/h36m/h36m_annot.tar")
            .headers(headers)
            .send().await
            .unwrap();

        let bytes = response.bytes().await.unwrap();

        // Write to file
        let mut file = std::fs::File::create("tmp.txt").unwrap();
        file.write_all(&bytes).unwrap();
    }

    #[test]
    fn test_split_data() {
        let data = Bytes::from("Hello, World!");
        let size = data.len() as u64;

        let ranges = split_data(size, 4);

        println!("{:?}", ranges);
    }

    #[test]
    fn test_threads() {
        // Create a file to write to
        let file = std::fs::OpenOptions::new().write(true).create(true).open("tmp.txt").unwrap();

        // Wrap the file in an Arc to allow it to be shared between threads
        let file = Arc::new(Mutex::new(file));

        let mut handles: Vec<std::thread::JoinHandle<()>> = vec![];
        for i in 0..10 {
            // Clone the file Arc
            let file = file.clone();

            let handle = std::thread::spawn(move || {
                // Sleep for a bit
                std::thread::sleep(Duration::from_millis(1000));

                // Aquire the lock
                let mut file = file.lock().unwrap();

                // Write to the file
                writeln!(file, "Hello from thread {}", i).unwrap();
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        println!("Hello from the main thread");
    }

    #[test]
    fn test_thread_pool() {
        let file_size: u64 = 1024;
        let ranges = split_data(file_size, 16);

        let pool = ThreadPoolBuilder::new().num_threads(16).build().unwrap();
        println!("Number of threads: {}", pool.current_num_threads());

        pool.install(|| {
            ranges.par_iter().for_each(|range| {
                // Sleep for a bit
                std::thread::sleep(Duration::from_millis(1000));

                println!("{:?}", range);
            });
        })
    }

    #[tokio::test]
    async fn test_tokio_thread_pool() -> Result<()> {
        let file_size: u64 = 1024;
        let ranges = split_data(file_size, 16);

        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
        let client = Arc::new(client);

        let tasks: Vec<_> = ranges
            .into_iter()
            .map(|range| {
                let client = Arc::clone(&client);

                tokio::task::spawn(async move {
                    // Sleep for a bit
                    tokio::time::sleep(Duration::from_millis(1000)).await;

                    println!("client: {:#?}", client);

                    // This is the async function that prints the range
                    println!("{:?}", range);
                })
            })
            .collect();

        for task in tasks {
            let _ = task.await.unwrap();
        }

        Ok(())
    }
}
