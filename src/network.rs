use reqwest::{Client, ClientBuilder};
use std::sync::Arc;

use reqwest::{
    cookie::{self},
    header,
};

pub fn create_spoof_client(
    cookie_provider: impl Into<Option<Arc<cookie::Jar>>>,
) -> Result<Client, reqwest::Error> {
    let headers = header::HeaderMap::from_iter([
            (
                header::USER_AGENT,
                header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36",
                )
            ),
            (
                header::ACCEPT,
                header::HeaderValue::from_static(
                    "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
                )
            ),
            (
                header::ACCEPT_LANGUAGE,
                header::HeaderValue::from_static("en-US,en;q=0.9"),
            )
        ]);
    let mut builder = ClientBuilder::new();

    if let Some(provider) = cookie_provider.into() {
        builder = builder.cookie_provider(provider);
    }

    builder
        .no_proxy()
        .default_headers(headers)
        .cookie_store(true)
        .build()
}
