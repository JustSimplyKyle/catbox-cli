use std::path::PathBuf;

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Top-level command.
pub struct Cli {
    #[argh(subcommand)]
    pub command: CliSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum CliSubCommands {
    File(FileCommand),
    Album(AlbumCommand),
    Config(ConfigCommand),
}

// Config Commands <------------------>

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Controling files.
#[argh(subcommand, name = "config")]
pub struct ConfigCommand {
    #[argh(subcommand)]
    pub command: ConfigSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum ConfigSubCommands {
    Save(SaveConfig),
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Fetchs the video from desired url.
#[argh(subcommand, name = "save")]
pub struct SaveConfig {
    #[argh(option)]
    /// your account user name
    pub username: String,
    /// your password
    #[argh(option)]
    pub password: String,
}

// <-------------------------------->
// File Commands <------------------>

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Controling files.
#[argh(subcommand, name = "file")]
pub struct FileCommand {
    #[argh(subcommand)]
    pub command: FileSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum FileSubCommands {
    Upload(FileUpload),
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Uploading files.
#[argh(subcommand, name = "upload")]
pub struct FileUpload {
    #[argh(positional)]
    /// file paths
    pub paths: Vec<PathBuf>,
}

// <--------------------------------->
// Ablum Commands <------------------>
#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Control your album
#[argh(subcommand, name = "album")]
pub struct AlbumCommand {
    #[argh(subcommand)]
    pub command: AlbumSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum AlbumSubCommands {
    Fetch(AlbumFetch),
    List(AlbumList),
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// Fetchs the video from desired url.
#[argh(subcommand, name = "fetch-files")]
pub struct AlbumFetch {
    #[argh(option)]
    /// the url of said album
    pub url: Option<String>,
    /// the short of said album(the last part of)
    #[argh(option)]
    pub short: Option<String>,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
/// List the album from the logined state.
#[argh(subcommand, name = "list")]
pub struct AlbumList {}

// <--------------------------------->
