use indicatif::ProgressBar;
use reqwest::Url;

use std::{path::Path, time::Duration};
use tokio::sync::OnceCell;

use tl::ParserOptions;

use crate::{
    album::Album,
    authentication::AuthenticatedClient,
    ensure, get_password_entry, get_username_entry,
    upload::{upload_file, UploadTarget},
};

use crate::errors::*;

#[derive(Clone)]
pub struct User {
    client: AuthenticatedClient,
    user_hash: OnceCell<String>,
}

pub const API_URL: &str = "https://catbox.moe/user/api.php";

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
        let target = UploadTarget::Catbox {
            user_hash: self.get_user_hash().await?,
        };
        upload_file(path, target, &self.client)
            .await
            .map_err(Into::into)
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
