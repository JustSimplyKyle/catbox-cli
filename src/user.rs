use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    multipart::{self, Part},
    Body, Url,
};

use futures_util::TryStreamExt;
use std::{path::Path, time::Duration};
use tokio::{fs::File, sync::OnceCell};

use tl::ParserOptions;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::{
    album::Album, authentication::AuthenticatedClient, ensure, get_password_entry,
    get_username_entry, MULTI_PROGRESS,
};

use crate::errors::*;

#[derive(Clone)]
pub struct User {
    client: AuthenticatedClient,
    user_hash: OnceCell<String>,
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
    pub async fn new() -> Result<Self, UserError> {
        let username = get_username_entry()?
            .get_password()
            .map_err(KeyringError::LackOfUser)?;
        let password = get_password_entry()?
            .get_password()
            .map_err(KeyringError::LackOfUser)?;

        let progress = ProgressBar::new_spinner();

        progress.enable_steady_tick(Duration::from_millis(200));

        progress.set_message("Initilizing user...");

        let client = AuthenticatedClient::new(&username, &password)
            .await
            .map_err(|source| UserError::AuthenticatedClientCreation { source, username })?;

        progress.finish_and_clear();

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
    ///
    /// # Panics
    ///
    /// Panics when the template provided to `ProgressBar` is invalid(compile time mistake)
    pub async fn upload_file(&self, path: impl AsRef<Path> + Send) -> Result<String, UserError> {
        let path = path.as_ref();

        let file = File::open(path)
            .await
            .map_err(|source| UserError::ReadFile {
                file: path.to_path_buf(),
                source,
            })?;

        let total_bytes = file
            .metadata()
            .await
            .map_err(|source| UserError::ReadFile {
                file: path.to_path_buf(),
                source,
            })?
            .len();

        let bar = ProgressBar::new(total_bytes).with_prefix(path.to_string_lossy().to_string());

        bar.set_style(
            #[allow(clippy::literal_string_with_formatting_args)]
            ProgressStyle::with_template(
                "{prefix:.magenta}\n[ETA: {eta}] [{decimal_bytes_per_sec:}] [{elapsed_precise}] {wide_bar:.cyan/blue} {decimal_bytes}/{decimal_total_bytes}",
            )
            .expect("Invalid template(compile time issue)")
            .progress_chars("##-"),
        );

        MULTI_PROGRESS.add(bar.clone());

        bar.enable_steady_tick(Duration::from_millis(500));

        let bar_cloned = bar.clone();

        let stream = FramedRead::new(file, BytesCodec::new()).inspect_ok(move |x| {
            bar_cloned.inc(x.len() as u64);
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

        let resp = self
            .client
            .post(API_URL)
            .multipart(form)
            .send()
            .await
            .map_err(NetworkError::DownloadRequest)?;

        let code = resp.status();

        let text = resp.text().await.map_err(NetworkError::InvalidText)?;

        if !code.is_success() {
            return Err(UserError::InvalidResponseWithCode { code, reason: text });
        }

        bar.finish_and_clear();
        Ok(text)
    }

    pub async fn upload_to_album(&self, album: &Album, slug: &str) -> Result<(), UserError> {
        let user_hash = self.get_user_hash().await?;

        let short = album
            .url
            .path_segments()
            .ok_or(UserError::ShortParsing {
                url: album.url.clone(),
            })?
            .nth(1)
            .ok_or(UserError::ShortParsing {
                url: album.url.clone(),
            })?;

        ensure!(
            self.fetch_uploaded_files()
                .await?
                .into_iter()
                .any(|x| &x.path()[1..] == slug),
            UserError::InvalidSlug {
                slug: slug.to_string()
            }
        );

        let resp = self
            .client
            .post(API_URL)
            .form(&[
                ("reqtype", "addtoalbum"),
                ("userhash", &user_hash),
                ("short", short),
                ("files", slug),
            ])
            .send()
            .await
            .map_err(NetworkError::DownloadRequest)?;

        let code = resp.status();

        ensure!(
            code.is_success(),
            UserError::InvalidResponseWithCode {
                code,
                reason: resp.text().await.map_err(NetworkError::InvalidText)?,
            }
        );

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
                let html = self.client.fetch_html(ACCOUNT_URL).await?;
                let html = tl::parse(&html, ParserOptions::default())
                    .map_err(HtmlParsingError::InvalidHtml)?;
                let parser = html.parser();

                let user_hash = html
                    .get_elements_by_class_name("notesmall")
                    .next()
                    .ok_or(HtmlParsingError::LackOfContainer)?
                    .get(parser)
                    .ok_or(HtmlParsingError::LackOfNodeid)?
                    .children()
                    .ok_or(HtmlParsingError::LackOfChildren)?
                    .all(parser)
                    .iter()
                    .filter_map(|x| x.children().and_then(|x| x.boundaries(parser)))
                    .filter_map(|(x, y)| {
                        Some((parser.resolve_node_id(x)?, parser.resolve_node_id(y)?))
                    })
                    .find(|(title, _)| title.inner_text(parser) == "Your userhash is:")
                    .map(|(_, body)| body.inner_text(parser).to_string())
                    .map(|x| x.trim_start().to_owned())
                    .ok_or(HtmlParsingError::LackOfUserHash)?;

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

        let html = self.client.fetch_html(ALBUM_VIEW_URL).await?;
        let html =
            tl::parse(&html, ParserOptions::default()).map_err(HtmlParsingError::InvalidHtml)?;
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
    pub async fn fetch_uploaded_files(&self) -> Result<Vec<Url>, UserError> {
        const USER_VIEW_URL: &str = "https://catbox.moe/user/view.php";

        let html = self.client.fetch_html(USER_VIEW_URL).await?;
        let html =
            tl::parse(&html, ParserOptions::default()).map_err(HtmlParsingError::InvalidHtml)?;
        let parser = html.parser();

        let files = html
            .get_element_by_id("results")
            .ok_or(HtmlParsingError::LackOfContainer)?
            .get(parser)
            .ok_or(HtmlParsingError::LackOfNodeid)?
            .children()
            .ok_or(HtmlParsingError::LackOfChildren)?
            .all(parser)
            .iter()
            .filter_map(|x| x.as_tag())
            .filter(|x| x.attributes().contains("target"))
            .filter_map(|x| x.attributes().get("href")??.try_as_utf8_str())
            .filter_map(|x| Url::parse(x).ok())
            .collect();
        Ok(files)
    }
}
