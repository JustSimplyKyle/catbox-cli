use super::errors::*;
use std::time::Duration;

use indicatif::ProgressBar;
use rand::{seq::SliceRandom, thread_rng};
use reqwest::Url;
use tl::ParserOptions;

use crate::network::create_spoof_client;

pub struct Files {
    pub urls: Vec<Url>,
}

impl Files {
    pub fn random_file(&self) -> Option<&str> {
        let mut rng = thread_rng();
        self.urls.choose(&mut rng).map(Url::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Album {
    pub url: Url,
}

impl Album {
    /// Creates a new `Album` instance.
    ///
    /// The `Album` struct stores the URL of an album.
    ///
    /// # Example
    ///
    /// ```
    /// let album = Album::new("https://catbox.moe/c/hpxdlu");
    /// ```
    pub fn new(url: impl Into<Url>) -> Self {
        Self { url: url.into() }
    }

    /// Fetches the the URLs from the album's webpage.
    ///
    /// This function sends an HTTP GET request to the album's URL, parses the
    /// HTML response, and extracts the URLs of the files embedded within the page.
    ///
    pub async fn fetch_files(&self) -> Result<Files, AlbumError> {
        let client = create_spoof_client(None)?;

        let pb = ProgressBar::new_spinner().with_message("Downloading data...");
        pb.enable_steady_tick(Duration::from_millis(100));

        let file = client
            .get(self.url.clone())
            .send()
            .await
            .map_err(NetworkError::DownloadRequest)?
            .error_for_status()
            .map_err(NetworkError::ErrorCode)?
            .text()
            .await
            .map_err(NetworkError::InvalidText)?;

        pb.finish_and_clear();

        let html =
            tl::parse(&file, ParserOptions::default()).map_err(HtmlParsingError::InvalidHtml)?;

        let parser = html.parser();

        let urls = html
            .get_elements_by_class_name("imagecontainer")
            .next()
            .ok_or(HtmlParsingError::LackOfContainer)?
            .get(parser)
            .ok_or(HtmlParsingError::LackOfNodeid)?
            .children()
            .ok_or(HtmlParsingError::LackOfChildren)?
            .all(parser)
            .iter()
            .filter_map(|x| x.as_tag())
            .map(|x| {
                let attrs = x.attributes();
                attrs
                    .get("src")
                    .or_else(|| attrs.get("href"))
                    .ok_or(HtmlParsingError::LackOfSrc)
            })
            .filter_map(Result::transpose)
            .map(|x| {
                x?.try_as_utf8_str()
                    .ok_or(HtmlParsingError::Utf8Incompatiable)
            })
            .map(|x| x.map(|x| Url::parse(x).ok()))
            .filter(|x| x.as_ref().is_ok_and(Option::is_some))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Files { urls })
    }
}
