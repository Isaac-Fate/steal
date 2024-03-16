use reqwest::{ header::{ HeaderMap, HeaderValue, USER_AGENT }, Client };
use crate::{ Result, Error };
use super::constants::USER_AGENT_VALUE;

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
