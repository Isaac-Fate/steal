use reqwest::{ header::{ HeaderMap, HeaderValue, USER_AGENT }, Client };
use crate::{ Result, Error, USER_AGENT_VALUE };

pub struct ResponseHeaderInfo {
    pub content_length: Option<u64>,
}

/// Get the response headers, and extract the information of interests.
pub async fn get_header_info(client: &Client, url: &str) -> Result<ResponseHeaderInfo> {
    // Get the headers
    let headers = get_headers(client, url).await?;

    // Get the content length
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    Ok(ResponseHeaderInfo { content_length })
}

/// Get the reponse headers as a `HeaderMap`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_headers() -> Result<()> {
        // Create a client
        let client = Client::new();

        // Get the headers
        let headers = get_headers(&client, "https://www.google.com").await?;

        println!("{:#?}", headers);

        Ok(())
    }
}
