use std::path::PathBuf;

use error_set::error_set;
use url::Url;

#[macro_export]
macro_rules! ensure {
    ($predicate:expr, $err:expr $(,)?) => {
        if !$predicate {
            return Err($err).map_err(::core::convert::Into::into);
        }
    };
}

error_set! {
    AppError = {
        #[display("Fails to output multi-progress bar")]
        MultiProgressOutputError(std::io::Error),
        #[display("Fails to translate to json")]
        JsonTranslationError(serde_json::Error),
        #[display("Invalid url. '{url}'")]
        InvalidUrl(url::ParseError) { url: String },
    }|| AlbumError || UserError;

    AlbumError = HtmlParsingError || NetworkError;
    UserError = InnerUserError || NetworkError || KeyringError || HtmlParsingError || UploadFileError;

    HtmlParsingError = {
        #[display("Fails to parse html.")]
        InvalidHtml(tl::ParseError),
        #[display("Fails to parse html. Reason: Lack of node id from vdom")]
        LackOfNodeid,
        #[display("Fails to parse html. Reason: Lack of `div` container")]
        LackOfContainer,
        #[display("Fails to parse html. Reason: Lack of children from the `div` container")]
        LackOfChildren,
        #[display("Fails to parse html. Reason: Lack of `src` from the `div` element")]
        LackOfSrc,
        #[display("Fails to parse html. Reason: Lacks user hash")]
        LackOfUserHash,
        #[display("Fails to parse html. Reason: `src` string is not utf8 compatiable")]
        Utf8Incompatiable,
    };

    NetworkError = {
        #[display("Fails to create reqwest client")]
        ClientCreation(reqwest::Error),
        #[display("Fails the request to target url: '{}'", source.url().map(ToString::to_string).unwrap_or_default())]
        DownloadRequest(reqwest::Error),
        #[display("Request returns non 200 error code: '{}'.", source.status().map(|x| x.as_u16()).unwrap_or_default())]
        ErrorCode(reqwest::Error),
        #[display("Downloaded file can not be turned into text")]
        InvalidText(reqwest::Error),
    };

    KeyringError = {
        #[display("Fails to initilize keyring instance.")]
        KeyringInitilization(keyring::Error),
        #[display("Lack of password, please set one with `cbx config save --password`!")]
        LackOfPassword(keyring::Error),
        #[display("Lack of user, please set one with `cbx config save --username`!")]
        LackOfUser(keyring::Error),
        #[display("Fails to save variable due to keyring error.")]
        FailureSettingVariable(keyring::Error)
    };

    InnerUserError = {
        #[display("Fails to create authenticated client with {username}")]
        AuthenticatedClientCreation(NetworkError) {
            username: String,
        },
        #[display("Slug({slug}) given can not be found in user profile")]
        InvalidSlug { slug: String },
        #[display("Fails to parse a short from url: {url}")]
        ShortParsing { url: Url },
    };

    UploadFileError = {
        #[display("Fails to read file `{}`", file.display())]
        ReadFile(std::io::Error) { file: PathBuf },
        #[display("Request returns non 200 error code: '{code}'.{}", ("\nReason: ".to_string() + reason))]
        InvalidResponseWithCode { code: reqwest::StatusCode, reason: String },
        #[display("Failed to determine filename for upload")]
        InvalidFilename
    } || NetworkError;

}
