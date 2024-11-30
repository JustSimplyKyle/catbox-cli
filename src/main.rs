pub mod album;
pub(crate) mod authentication;
mod cli;
pub(crate) mod network;
pub mod user;

use std::{path::PathBuf, sync::Arc, time::Duration};

use cli::*;

use album::Album;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar};
use keyring::Entry;
use reqwest::Url;
use tokio::sync::OnceCell;
use user::{User, UserError};

fn get_username_entry() -> keyring::Result<Entry> {
    Entry::new("catbox-cli", "username")
}

fn get_password_entry() -> keyring::Result<Entry> {
    Entry::new("catbox-cli", "password")
}

struct UserInstance {
    cache: OnceCell<User>,
}

impl UserInstance {
    pub fn new() -> Self {
        Self {
            cache: OnceCell::new(),
        }
    }
    pub async fn get(&self) -> Result<&User, UserError> {
        self.cache.get_or_try_init(User::new).await
    }
}

/// Album Control
#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli: Cli = argh::from_env();

    let m = MultiProgress::new();

    let user_instance = Arc::new(UserInstance::new());

    let upload_files = |paths: Vec<PathBuf>| {
        let m = m.clone();
        let user_instance = user_instance.clone();
        async move {
            let user = user_instance.get().await?;

            futures_util::stream::iter(paths)
                .map(|x| {
                    user.upload_file(x.clone(), m.clone())
                        .map(move |y| Ok::<_, color_eyre::Report>((x, y?)))
                })
                .buffer_unordered(5)
                .inspect_ok(|(path, url)| {
                    m.println(format!("{}: {url}", path.display()))
                        .expect("Can't print using indicaitf");
                })
                .map_ok(|x| x.1)
                .try_collect::<Vec<_>>()
                .await
        }
    };

    let add_to_album = |album: String, files: Vec<String>| {
        let m = m.clone();
        let user_instance = user_instance.clone();
        async move {
            let user = user_instance.get().await?;

            let album = {
                if album.contains("catbox.moe") {
                    Album::new(Url::parse(&album)?)
                } else {
                    Album::new(Url::parse(&format!("https://catbox.moe/c/{album}"))?)
                }
            };

            futures_util::stream::iter(files.into_iter().filter_map(|x| {
                if x.contains("files.catbox.moe") {
                    Some(Url::parse(&x).ok()?.path_segments()?.next()?.to_owned())
                } else {
                    Some(x)
                }
            }))
            .map(move |x| {
                let user = user.clone();
                let album = album.clone();
                let pb = ProgressBar::new_spinner();
                m.add(pb.clone());

                pb.enable_steady_tick(Duration::from_millis(100));

                pb.set_message(format!("Uploading '{x}' to album"));

                async move {
                    user.upload_to_album(&album, &x).await?;

                    pb.finish_and_clear();
                    Ok::<_, UserError>(())
                }
            })
            .buffer_unordered(5)
            .try_collect::<Vec<_>>()
            .await?;
            Ok::<_, color_eyre::Report>(())
        }
    };

    match cli.command {
        CliSubCommands::File(FileCommand {
            command: FileSubCommands::Upload(FileUpload { paths }),
        }) => {
            upload_files(paths).await?;
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::Fetch(AlbumFetch { album }),
        }) => {
            let album = {
                if album.contains("catbox.moe") {
                    Album::new(Url::parse(&album)?)
                } else {
                    Album::new(Url::parse(&format!("https://catbox.moe/c/{album}"))?)
                }
            };

            for (i, x) in album
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
            command: AlbumSubCommands::Add(AddFiles { album, files }),
        }) => {
            add_to_album(album, files).await?;
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::Upload(UploadFiles { album, files }),
        }) => {
            let urls = upload_files(files).await?;

            add_to_album(album, urls).await?;
        }
        CliSubCommands::Album(AlbumCommand {
            command: AlbumSubCommands::List(AlbumList {}),
        }) => {
            let user = user_instance.get().await?;

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
        CliSubCommands::Config(ConfigCommand {
            command: ConfigSubCommands::Delete(DeleteConfig {}),
        }) => {
            get_username_entry()?.delete_credential()?;
            get_password_entry()?.delete_credential()?;
        }
    }

    Ok(())
}
