use std::ops::Deref;

use reqwest::Client;
use snafu::{ResultExt, Snafu};

use crate::network::create_spoof_client;

#[derive(Snafu, Debug)]
pub enum AuthenticationError {
    #[snafu(display("Downloaded Html file can not be turned into text"))]
    NotAText { source: reqwest::Error },
    #[snafu(display("Fails to create reqwest client"))]
    ClientCreation { source: reqwest::Error },
    #[snafu(display("Request to target failed. url: '{url}'"))]
    Request { url: String, source: reqwest::Error },
    #[snafu(display("Request returns non 200 error code: '{}'", source.status().map(|x| x.as_u16()).unwrap_or_default()))]
    ErrorCode { source: reqwest::Error },
}

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
    pub async fn new(username: &str, password: &str) -> Result<Self, AuthenticationError> {
        const LOGIN_URL: &str = "https://catbox.moe/user/dologin.php";

        let client = create_spoof_client(None).context(ClientCreationSnafu)?;

        client
            .post(LOGIN_URL)
            .form(&[("username", username), ("password", password)])
            .send()
            .await
            .context(RequestSnafu { url: LOGIN_URL })?
            .error_for_status()
            .context(ErrorCodeSnafu)?;
        Ok(Self { client })
    }

    pub async fn fetch_html(&self, url: &str) -> Result<String, AuthenticationError> {
        self.client
            .get(url)
            .send()
            .await
            .context(RequestSnafu { url })?
            .error_for_status()
            .context(ErrorCodeSnafu)?
            .text()
            .await
            .context(NotATextSnafu)
    }
}
