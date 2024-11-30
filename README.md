# Overview
`catbox-cli` is a simple cli tool that uploads to `catbox.moe`. It differs from other solution is that is not simply an api wrapper. It also acts as an alternative **front end** to `catbox.moe`, allowing you to do more things than the api allows.

# Why `catbox-cli`
Compared to other catbox cli providers, it has the following benefits.
- a progress bar, crucial for uploading large files
- the ability to upload multiple files at once
- simple interface, mostly utilizing `argh::Command`  

# Usage
## Authentication
This is vital for `catbox-cli`, as it does not use the traditional `CATBOX_USER_HASH` for authentication. It uses cookies to authenticate, so you will have to provide your username and password to `catbox-cli`. Your credentials are **not** stored in plain text, instead guarded by your system's integrated password storer, which supports MacOs, Windows, and Linux.

Use the following line to add credentials for `catbox-cli` to use.
`cbx config save --username [your_user_name] --password [your_pass_word]`

If you want to delete your credentials, simply type:
`cbx config delete`

## Uploading files
For uploading files, type:
`cbx file upload [file1] [file2] [file3]`
The aforementioned progress bar can be seen here!
![image](https://github.com/user-attachments/assets/e76e50a0-de47-44d0-9c7e-394615c3dd47)

## Listing files you have uploaded
You can list all the files you have uploaded with:
`cbx file list`

## Listing albums created by you
Listing albums that were created by you is as simple as:
`cbx album list`

## Adding an existing file from `catbox.moe` to an album
For example, adding `w0v6bk.webm` and `7mc3en.pdf` to album `hpxdlu`:
`cbx album w0v6bk.webm 7mc3en.pdf --album hpxdlu`

This will **error** when the given file is not found in your user profile.

## Adding an non-exsistent file to an album
Sometimes you just want to add files that are your computer to an album.
You can accomplish this with `cbx file upload` paired with `cbx album add`, but it's quite cumbersome.
`catbox-cli` provides the following command to simply uploading to an album:
`cbx album upload [file1] [file2] --album [album_slug]`
