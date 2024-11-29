pub mod album;
pub(crate) mod authentication;
mod cli;
pub(crate) mod network;
pub mod user;

use std::time::Duration;

use cli::*;

use album::Album;
use color_eyre::eyre::bail;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar};
use keyring::Entry;
use reqwest::Url;
use user::{User, UserError};

fn get_username_entry() -> keyring::Result<Entry> {
    Entry::new("catbox-cli", "username")
}

fn get_password_entry() -> keyring::Result<Entry> {
    Entry::new("catbox-cli", "password")
}

/// Album Control
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli: Cli = argh::from_env();

    let m = MultiProgress::new();

    match cli.command {
        CliSubCommands::File(FileCommand {
            command: FileSubCommands::Upload(FileUpload { paths }),
        }) => {
            let user = User::new().await?;

            let mut stream = futures_util::stream::iter(paths)
                .map(|x| {
                    user.upload_file(x.clone(), m.clone())
                        .map(move |y| Ok::<_, color_eyre::Report>((x, y?)))
                })
                .buffer_unordered(5);

            while let Some(x) = stream.next().await {
                let (path, url) = x?;

                m.println(format!("{}: {url}", path.display()))?;
            }
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::Fetch(AlbumFetch { url, short }),
        }) => {
            let url = match (url, short) {
                (None, None) => {
                    bail!("you must provide a url or a short!");
                }
                (Some(_), Some(_)) => {
                    bail!("you can't provide both url and short!");
                }
                (None, Some(short)) => {
                    format!("https://catbox.moe/c/{short}")
                }
                (Some(url), None) => url,
            };

            for (i, x) in Album::new(Url::parse(&url)?)
                .fetch_files()
                .await?
                .urls
                .into_iter()
                .rev()
                .enumerate()
            {
                println!("File {}: {x}", i + 1);
            }
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::Add(AddFiles { short, files }),
        }) => {
            let user = User::new().await?;
            let album = Album::new(Url::parse(&format!("https://catbox.moe/c/{short}"))?);

            futures_util::stream::iter(files.into_iter().filter_map(|x| {
                if x.contains("files.catbox.moe") {
                    Some(Url::parse(&x).ok()?.path_segments()?.nth(1)?[1..].to_string())
                } else {
                    Some(x)
                }
            }))
            .map(move |x| {
                let value = user.clone();
                let album = album.clone();
                let pb = ProgressBar::new_spinner();
                m.add(pb.clone());

                pb.enable_steady_tick(Duration::from_millis(100));

                pb.set_message(format!("Uploading '{x}' to album"));

                async move {
                    value.upload_to_album(&album, &x).await?;

                    pb.finish_and_clear();
                    Ok::<_, UserError>(())
                }
            })
            .buffer_unordered(5)
            .try_collect::<Vec<_>>()
            .await?;
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::List(AlbumList {}),
        }) => {
            let user = User::new().await?;

            for (i, x) in user.fetch_albums().await?.into_iter().rev().enumerate() {
                println!("Album {}: {}", i + 1, x.url);
            }
        }
        CliSubCommands::Config(ConfigCommand {
            command: ConfigSubCommands::Save(SaveConfig { username, password }),
        }) => {
            get_username_entry()?.set_password(&username)?;
            get_password_entry()?.set_password(&password)?;
        }
    }

    Ok(())
}
