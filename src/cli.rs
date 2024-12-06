use std::path::PathBuf;

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Top-level command.
pub struct Cli {
    #[argh(subcommand)]
    pub command: CliSubCommands,
    #[argh(switch, short = 'j')]
    /// whether to output in json
    pub json: bool,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
#[argh(subcommand)]
pub enum CliSubCommands {
    File(FileCommand),
    Album(AlbumCommand),
    Config(ConfigCommand),
}

// Config Commands <------------------>

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Controling files.
#[argh(subcommand, name = "config")]
pub struct ConfigCommand {
    #[argh(subcommand)]
    pub command: ConfigSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
#[argh(subcommand)]
pub enum ConfigSubCommands {
    Save(SaveConfig),
    Delete(DeleteConfig),
}
#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Deletes both your account username and password.
#[argh(subcommand, name = "delete")]
pub struct DeleteConfig {}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Saves your account username and password.
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

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Controling files.
#[argh(subcommand, name = "file")]
pub struct FileCommand {
    #[argh(subcommand)]
    pub command: FileSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
#[argh(subcommand)]
pub enum FileSubCommands {
    Upload(FileUpload),
    List(FileList),
}
#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Uploading files.
#[argh(subcommand, name = "list")]
pub struct FileList {}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Uploading files.
#[argh(subcommand, name = "upload")]
pub struct FileUpload {
    #[argh(positional)]
    /// file paths
    pub paths: Vec<PathBuf>,
}

// <--------------------------------->
// Album Commands <------------------>
#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Control your album
#[argh(subcommand, name = "album")]
pub struct AlbumCommand {
    #[argh(subcommand)]
    pub command: AlbumSubCommands,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
#[argh(subcommand)]
pub enum AlbumSubCommands {
    List(AlbumList),
    Add(AddFiles),
    Upload(UploadFiles),
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Adding files via their short ids(allows url input) to the album.
#[argh(subcommand, name = "add")]
pub struct AddFiles {
    /// the short of said album(the last part of the url)
    #[argh(option)]
    pub album: String,
    #[argh(positional)]
    /// files to add to album
    pub files: Vec<String>,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// Uploading files via their short ids(allows url input) to said album.
#[argh(subcommand, name = "upload")]
pub struct UploadFiles {
    /// the short of said album(the last part of the url)
    #[argh(option)]
    pub album: String,
    #[argh(positional)]
    /// files to add to album
    pub files: Vec<PathBuf>,
}

#[derive(FromArgs, PartialEq, Eq, Debug, Clone)]
/// List all the albums from the logined state.
/// if the `album` option is given, it will list the files of the album instead
#[argh(subcommand, name = "list")]
pub struct AlbumList {
    #[argh(option)]
    /// the short of the album(the last part of the url)
    pub album: Option<String>,
}

// <--------------------------------->
