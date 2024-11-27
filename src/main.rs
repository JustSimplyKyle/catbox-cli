pub mod album;
pub(crate) mod authentication;
pub(crate) mod network;
pub mod user;

use user::User;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let user = User::new("simplykyle", "u89ccNFULbS1").await?;

    for (i, x) in user.fetch_uploaded_files().await?.into_iter().enumerate() {
        println!("file {}: {x}", i + 1);
    }

    for (i, x) in user.fetch_albums().await?.into_iter().enumerate() {
        println!("album {}: {}", i + 1, x.url);
    }

    println!("{}", user.get_user_hash().await?);

    println!("url: {}", user.upload_file("./ocean.mp4").await?);

    Ok(())
}
