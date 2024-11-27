use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    multipart::{self, Part},
    Body, Url,
};

use futures_util::TryStreamExt;
use snafu::{OptionExt, ResultExt, Snafu};
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{fs::File, sync::OnceCell};

use tl::ParserOptions;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::{
    album::Album,
    authentication::{AuthenticatedClient, AuthenticationError},
};

pub struct User {
    client: AuthenticatedClient,
    user_hash: OnceCell<String>,
}

#[derive(Snafu, Debug)]
pub enum UserError {
    #[snafu(display("Fails to create authenticated client with {username} and {password}"))]
    AuthenticatedClientCreation {
        username: String,
        password: String,
        source: AuthenticationError,
    },
    #[snafu(display("Request to target failed. url: '{url}'"))]
    Request { url: String, source: reqwest::Error },
    #[snafu(display("Return to response can't be parsed as text"))]
    NotAText { source: reqwest::Error },
    #[snafu(display("Request returns non 200 error code: '{}'", source.status().map(|x| x.as_u16()).unwrap_or_default()))]
    ErrorCode { source: reqwest::Error },
    #[snafu(display("Fails to read file `{}`", file.display()))]
    ReadFile { file: PathBuf, source: io::Error },

    #[snafu(display("Downloaded html file is not valid"))]
    InvalidHtml { source: AuthenticationError },
    #[snafu(display("Fails to parse html. html: {html}"))]
    HtmlParse {
        html: String,
        source: tl::ParseError,
    },
    #[snafu(display("Fails to parse html. Reason: Lack of node id from vdom(impossible!)"))]
    LackOfNodeid,
    #[snafu(display("Fails to parse html. Reason: Lack of preview container"))]
    LackOfContainer,
    #[snafu(display("Fails to parse html. Reason: Lack of children from the video container"))]
    LackOfChildren,
    #[snafu(display("Fails to parse html. Reason: Lack of user hash"))]
    LackOfUserHash,
}

const API_URL: &str = "https://catbox.moe/user/api.php";

impl User {
    /// Creates a new `User` instance.
    ///
    /// The `User` struct stores the authenticated client that provides functions.
    ///
    /// # Example
    ///
    /// ```
    /// let user = User::new("kyle", "some_password");
    /// ```
    pub async fn new(username: &str, password: &str) -> Result<Self, UserError> {
        let client = AuthenticatedClient::new(username, password)
            .await
            .context(AuthenticatedClientCreationSnafu { username, password })?;
        Ok(Self {
            client,
            user_hash: OnceCell::new(),
        })
    }

    /// Uploads the file using `User`.
    ///
    /// # Example
    ///
    /// ```
    /// let user = User::new("kyle", "some_password");
    /// user.upload_file("./happy.mp4").await?;
    /// ```
    pub async fn upload_file(&self, path: impl AsRef<Path> + Send) -> Result<String, UserError> {
        let path = path.as_ref();

        let file = File::open(path)
            .await
            .context(ReadFileSnafu { file: path })?;

        let total_bytes = file
            .metadata()
            .await
            .context(ReadFileSnafu { file: path })?
            .len();

        let bar = ProgressBar::new(total_bytes);
        bar.set_style(
            ProgressStyle::with_template(
                "[{decimal_bytes_per_sec:}] [{elapsed_precise}] {wide_bar:.cyan/blue} {decimal_bytes}/{decimal_total_bytes} ETA: {eta}",
            )
            .expect("Invalid template(compile time issue)")
            .progress_chars("##-"),
        );

        let stream = FramedRead::new(file, BytesCodec::new()).inspect_ok(move |x| {
            bar.inc(x.len() as u64);
        });

        let body_stream = Body::wrap_stream(stream);

        let hash = self.get_user_hash().await?;

        let form = multipart::Form::new()
            .text("reqtype", "fileupload")
            .text("userhash", hash)
            .part(
                "fileToUpload",
                Part::stream_with_length(body_stream, total_bytes)
                    .file_name(path.to_string_lossy().to_string()),
            );

        self.client
            .post(API_URL)
            .multipart(form)
            .send()
            .await
            .context(RequestSnafu {
                url: path.to_string_lossy(),
            })?
            .error_for_status()
            .context(ErrorCodeSnafu)?
            .text()
            .await
            .context(NotATextSnafu)
    }

    pub async fn upload_to_album(&self, album: Album, slug: String) -> Result<(), UserError> {
        let user_hash = self.get_user_hash().await?;

        let short = album.url.path();

        println!("Album short: {short}");

        self.client
            .post(API_URL)
            .header("reqtype", "addtoalbum")
            .header("userhash", user_hash)
            .header("short", short)
            .header("files", &slug)
            .send()
            .await
            .context(RequestSnafu { url: API_URL })?
            .error_for_status()
            .context(ErrorCodeSnafu)?;
        Ok(())
    }

    /// Gets the user hash of a `User`.
    ///
    /// # Example
    ///
    /// ```
    /// let user = User::new("kyle", "some_password");
    /// ```
    pub async fn get_user_hash(&self) -> Result<String, UserError> {
        const ACCOUNT_URL: &str = "https://catbox.moe/user/manage.php";
        self.user_hash
            .get_or_try_init(move || async move {
                let html = self
                    .client
                    .fetch_html(ACCOUNT_URL)
                    .await
                    .context(InvalidHtmlSnafu)?;

                let html = tl::parse(&html, ParserOptions::default())
                    .context(HtmlParseSnafu { html: &html })?;
                let parser = html.parser();

                let user_hash = html
                    .get_elements_by_class_name("notesmall")
                    .next()
                    .context(LackOfContainerSnafu)?
                    .get(parser)
                    .context(LackOfNodeidSnafu)?
                    .children()
                    .context(LackOfChildrenSnafu)?
                    .all(parser)
                    .iter()
                    .filter_map(|x| x.children().and_then(|x| x.boundaries(parser)))
                    .filter_map(|(x, y)| {
                        Some((parser.resolve_node_id(x)?, parser.resolve_node_id(y)?))
                    })
                    .find(|(title, _)| title.inner_text(parser) == "Your userhash is:")
                    .map(|(_, body)| body.inner_text(parser).to_string())
                    .map(|x| x.trim_start().to_owned())
                    .context(LackOfUserHashSnafu)?;

                Ok(user_hash)
            })
            .await
            .map(ToOwned::to_owned)
    }

    /// Lists the albums created by a `User`.
    ///
    /// # Example
    ///
    /// ```
    /// let user = User::new("kyle", "some_password");
    /// let albums = user.fetch_albums(&self).await?;
    /// ```
    pub async fn fetch_albums(&self) -> Result<Vec<Album>, UserError> {
        const ALBUM_VIEW_URL: &str = "https://catbox.moe/user/manage_albums.php";

        let html = self
            .client
            .fetch_html(ALBUM_VIEW_URL)
            .await
            .context(InvalidHtmlSnafu)?;

        let html =
            tl::parse(&html, ParserOptions::default()).context(HtmlParseSnafu { html: &html })?;
        let parser = html.parser();

        let Some(texts) = html.query_selector("span.textHolder") else {
            return Ok(vec![]);
        };

        let albums = texts
            .filter_map(|x| x.get(parser))
            .map(|x| x.inner_text(parser))
            .filter_map(|x| Url::parse(&x).ok())
            .map(Album::new)
            .collect();

        Ok(albums)
    }

    /// Lists the files created by a `User`.
    ///
    /// # Example
    ///
    /// ```
    /// let user = User::new("kyle", "some_password");
    /// let albums = user.fetch_uploaded_files(&self).await?;
    /// ```
    pub async fn fetch_uploaded_files(&self) -> Result<Vec<String>, UserError> {
        const USER_VIEW_URL: &str = "https://catbox.moe/user/view.php";

        let html = self
            .client
            .fetch_html(USER_VIEW_URL)
            .await
            .context(InvalidHtmlSnafu)?;

        let html =
            tl::parse(&html, ParserOptions::default()).context(HtmlParseSnafu { html: &html })?;
        let parser = html.parser();

        let videos = html
            .get_element_by_id("results")
            .context(LackOfContainerSnafu)?
            .get(parser)
            .context(LackOfNodeidSnafu)?
            .children()
            .context(LackOfChildrenSnafu)?
            .all(parser)
            .iter()
            .filter_map(|x| x.as_tag())
            .filter(|x| x.attributes().contains("target"))
            .filter_map(|x| x.attributes().get("href")??.try_as_utf8_str())
            .map(ToString::to_string)
            .collect();
        Ok(videos)
    }
}
