use rand::{seq::SliceRandom, thread_rng};
use reqwest::Url;
use snafu::{OptionExt, ResultExt, Snafu};
use tl::ParserOptions;

use crate::network::create_spoof_client;

#[derive(Snafu, Debug)]
pub enum AlbumError {
    #[snafu(display("Fails to create reqwest client"))]
    ClientCreation { source: reqwest::Error },
    #[snafu(display("Request to target failed. url: '{url}'"))]
    Request { url: String, source: reqwest::Error },
    #[snafu(display("Request returns non 200 error code: '{}'", source.status().map(|x| x.as_u16()).unwrap_or_default()))]
    ErrorCode { source: reqwest::Error },
    #[snafu(display("Fails to parse html. html: {html}"))]
    HtmlParse {
        html: String,
        source: tl::ParseError,
    },
    #[snafu(display("Fails to parse html. Reason: Lack of node id from vdom(impossible!)"))]
    LackOfNodeid,
    #[snafu(display("Fails to parse html. Reason: Lack of video container"))]
    LackOfVideoContainer,
    #[snafu(display("Fails to parse html. Reason: Lack of children from the video container"))]
    LackOfChildren,
    #[snafu(display("Fails to parse html. Reason: Lack of `src` from the video element"))]
    LackOfSrc,
    #[snafu(display("Fails to parse html. Reason: `src` string is not utf8 compatiable"))]
    Utf8Incompatiable,
    #[snafu(display("Downloaded Html file can not be turned into text"))]
    NotAText { source: reqwest::Error },
}

pub struct Videos {
    pub urls: Vec<String>,
}

impl Videos {
    pub fn random_video(&self) -> Option<&str> {
        let mut rng = thread_rng();
        self.urls.choose(&mut rng).map(String::as_str)
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

    /// Fetches the video URLs from the album's webpage.
    ///
    /// This function sends an HTTP GET request to the album's URL, parses the
    /// HTML response, and extracts the URLs of the videos embedded within the page.
    ///
    pub async fn fetch_videos(&self) -> Result<Videos, AlbumError> {
        let client = create_spoof_client(None).context(ClientCreationSnafu)?;
        let file = client
            .get(self.url.clone())
            .send()
            .await
            .context(RequestSnafu {
                url: self.url.clone(),
            })?
            .error_for_status()
            .context(ErrorCodeSnafu)?
            .text()
            .await
            .context(NotATextSnafu)?;

        let html =
            tl::parse(&file, ParserOptions::default()).context(HtmlParseSnafu { html: &file })?;

        let parser = html.parser();

        let urls = html
            .get_elements_by_class_name("imagecontainer")
            .next()
            .context(LackOfVideoContainerSnafu)?
            .get(parser)
            .context(LackOfNodeidSnafu)?
            .children()
            .context(LackOfChildrenSnafu)?
            .all(parser)
            .iter()
            .filter_map(|x| x.as_tag())
            .map(|x| x.attributes().get("src").context(LackOfSrcSnafu))
            .filter_map(Result::transpose)
            .map(|x| x?.try_as_utf8_str().context(Utf8IncompatiableSnafu))
            .map(|x| x.map(ToString::to_string))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Videos { urls })
    }
}
