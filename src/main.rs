pub mod album;
pub(crate) mod authentication;
pub(crate) mod network;
pub mod user;

use std::path::PathBuf;

use argh::FromArgs;
use futures_util::{FutureExt, StreamExt};
use indicatif::MultiProgress;
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
    #[argh(positional)]
    /// file path
    path: Vec<PathBuf>,
}

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
        CliSubCommands::File(FileCommand { command }) => match command {
            FileSubCommands::Upload(FileUpload { paths }) => {
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
        },
        CliSubCommands::Album(album_sub_command) => todo!(),
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
