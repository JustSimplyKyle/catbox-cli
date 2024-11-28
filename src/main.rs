pub mod album;
pub(crate) mod authentication;
mod cli;
pub(crate) mod network;
pub mod user;

use cli::*;

use album::Album;
use color_eyre::eyre::bail;
use futures_util::{FutureExt, StreamExt};
use indicatif::MultiProgress;
use reqwest::Url;
use user::User;

/// Album Control
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli: Cli = argh::from_env();

    println!("Initilizing user...");

    let user = User::new("simplykyle", "u89ccNFULbS1").await?;

    println!("Finished initilizing user!\n");

    let m = MultiProgress::new();

    match cli.command {
        CliSubCommands::File(FileCommand {
            command: FileSubCommands::Upload(FileUpload { paths }),
        }) => {
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
                .fetch_videos()
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
            command: AlbumSubCommands::List(AlbumList {}),
        }) => {
            for (i, x) in user.fetch_albums().await?.into_iter().rev().enumerate() {
                println!("Album {}: {}", i + 1, x.url);
            }
        }
        CliSubCommands::Config(ConfigCommand {
            command: ConfigSubCommands::Save(SaveConfig { username, password }),
        }) => {}
    }

    // println!("url: {}", user.upload_file(cli.command).await?);

    // for (i, x) in user.fetch_uploaded_files().await?.into_iter().enumerate() {
    //     println!("file {}: {x}", i + 1);
    // }

    // for (i, x) in user.fetch_albums().await?.into_iter().enumerate() {
    //     user.upload_to_album(&x, "6r38xu.pdf").await?;
    //     println!("album {}: {}", i + 1, x.url);
    // }

    // println!("{}", user.get_user_hash().await?);

    Ok(())
}
