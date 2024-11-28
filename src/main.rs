pub mod album;
pub(crate) mod authentication;
pub(crate) mod network;
pub mod user;

use std::path::PathBuf;

use album::Album;
use argh::FromArgs;
use color_eyre::eyre::bail;
use futures_util::{FutureExt, StreamExt};
use indicatif::MultiProgress;
use reqwest::Url;
use user::User;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
struct Cli {
    #[argh(subcommand)]
    command: CliSubCommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum CliSubCommands {
    File(FileCommand),
    Album(AlbumCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Controling files.
#[argh(subcommand, name = "file")]
struct FileCommand {
    #[argh(subcommand)]
    command: FileSubCommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum FileSubCommands {
    Upload(FileUpload),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Uploading files.
#[argh(subcommand, name = "upload")]
struct FileUpload {
    #[argh(positional)]
    /// file paths
    paths: Vec<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Control your album
#[argh(subcommand, name = "album")]
struct AlbumCommand {
    #[argh(subcommand)]
    command: AlbumSubCommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum AlbumSubCommands {
    Fetch(AlbumFetch),
    List(AlbumList),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Fetchs the video from desired url.
#[argh(subcommand, name = "fetch-files")]
struct AlbumFetch {
    #[argh(option)]
    /// the url of said album
    url: Option<String>,
    /// the short of said album(the last part of)
    #[argh(option)]
    short: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Fetchs the video from desired url.
#[argh(subcommand, name = "list")]
struct AlbumList {}

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
