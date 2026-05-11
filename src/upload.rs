use std::{path::Path, str::FromStr, time::Duration};

use futures_util::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    multipart::{self, Part},
    Body, Client,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{network::create_spoof_client, user, NetworkError, UploadFileError, MULTI_PROGRESS};

const LITTER_API_URL: &str = "https://litterbox.catbox.moe/resources/internals/api.php";

pub enum UploadTarget {
    Catbox { user_hash: String },
    Litterbox { expiry: LitterExpiry },
}

impl LitterExpiry {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::OneHour => "1h",
            Self::TwelveHours => "12h",
            Self::OneDay => "24h",
            Self::ThreeDays => "72h",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LitterExpiry {
    OneHour,
    TwelveHours,
    OneDay,
    ThreeDays,
}

impl FromStr for LitterExpiry {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "1h" => Ok(Self::OneHour),
            "12h" => Ok(Self::TwelveHours),
            "24h" => Ok(Self::OneDay),
            "72h" => Ok(Self::ThreeDays),
            s => Err(format!(
                "invalid expiry `{s}` (expected one of: 1h, 12h, 24h, 72h)"
            )),
        }
    }
}

pub async fn upload_file(
    path: impl AsRef<Path> + Send,
    target: UploadTarget,
    client: &Client,
) -> Result<String, UploadFileError> {
    let path = path.as_ref();

    let file = File::open(path)
        .await
        .map_err(|source| UploadFileError::ReadFile {
            file: path.to_path_buf(),
            source,
        })?;

    let total_bytes = file
        .metadata()
        .await
        .map_err(|source| UploadFileError::ReadFile {
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

    let stream = ReaderStream::new(file).inspect_ok(move |x| {
        bar_cloned.inc(x.len() as u64);
    });

    let body_stream = Body::wrap_stream(stream);

    let mut form = multipart::Form::new().text("reqtype", "fileupload").part(
        "fileToUpload",
        Part::stream_with_length(body_stream, total_bytes).file_name(
            path.file_name()
                .ok_or(UploadFileError::InvalidFilename)?
                .to_string_lossy()
                .to_string(),
        ),
    );

    let api = match target {
        UploadTarget::Catbox { user_hash } => {
            form = form.text("userhash", user_hash);
            user::API_URL
        }
        UploadTarget::Litterbox { expiry } => {
            form = form.text("time", expiry.as_str());
            LITTER_API_URL
        }
    };

    let resp = client
        .post(api)
        .multipart(form)
        .send()
        .await
        .map_err(NetworkError::DownloadRequest)?;

    let code = resp.status();

    let text = resp.text().await.map_err(NetworkError::InvalidText)?;

    if !code.is_success() {
        return Err(UploadFileError::InvalidResponseWithCode { code, reason: text });
    }

    bar.finish_and_clear();
    Ok(text)
}

pub async fn upload_temp_file(
    path: impl AsRef<Path> + Send,
    expiry: LitterExpiry,
) -> Result<String, UploadFileError> {
    let client = create_spoof_client(None)?;
    upload_file(path, UploadTarget::Litterbox { expiry }, &client).await
}
