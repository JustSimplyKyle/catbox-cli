use std::ops::Deref;

use crate::{network::create_spoof_client, NetworkError};
use reqwest::Client;

#[derive(Clone)]
pub struct AuthenticatedClient {
    client: Client,
}

impl Deref for AuthenticatedClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl AuthenticatedClient {
    pub async fn new(username: &str, password: &str) -> Result<Self, NetworkError> {
        const LOGIN_URL: &str = "https://catbox.moe/user/dologin.php";

        let client = create_spoof_client(None)?;

        client
            .post(LOGIN_URL)
            .form(&[("username", username), ("password", password)])
            .send()
            .await
            .map_err(NetworkError::DownloadRequest)?
            .error_for_status()
            .map_err(NetworkError::ErrorCode)?;
        Ok(Self { client })
    }

    pub async fn fetch_html(&self, url: &str) -> Result<String, NetworkError> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(NetworkError::DownloadRequest)?
            .error_for_status()
            .map_err(NetworkError::ErrorCode)?
            .text()
            .await
            .map_err(NetworkError::InvalidText)
    }
}
