mod error;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use futures::StreamExt;
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::fs::{self, FsError};
use std::env::consts;
use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{cmp, env};
use system_env::SystemLibc;
use tracing::{instrument, trace};
#[cfg(unix)]
use unix::*;
#[cfg(windows)]
use windows::*;

pub use error::ProtoInstallerError;

#[instrument]
pub fn determine_triple() -> miette::Result<String> {
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!(
            "{arch}-unknown-linux-{}",
            if SystemLibc::is_musl() { "musl" } else { "gnu" }
        ),
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_owned(),
        (os, arch) => {
            return Err(ProtoInstallerError::InvalidPlatform {
                arch: arch.to_owned(),
                os: os.to_owned(),
            }
            .into());
        }
    };

    Ok(target)
}

#[derive(Debug)]
pub struct DownloadResult {
    pub archive_file: PathBuf,
    pub file: String,
    pub file_stem: String,
    pub url: String,
}

#[instrument(skip(on_chunk))]
pub async fn download_release(
    triple: &str,
    version: &str,
    temp_dir: impl AsRef<Path> + Debug,
    on_chunk: impl Fn(u64, u64),
) -> miette::Result<DownloadResult> {
    let target_ext = if cfg!(windows) { "zip" } else { "tar.xz" };
    let target_file = format!("proto_cli-{triple}");

    let download_file = format!("{target_file}.{target_ext}");
    let download_url =
        format!("https://github.com/moonrepo/proto/releases/download/v{version}/{download_file}");

    trace!(
        version,
        triple,
        "Downloading proto release from {}",
        color::url(&download_url)
    );

    // Request file from url
    let handle_error = |error: reqwest::Error| ProtoInstallerError::DownloadFailed {
        url: download_url.clone(),
        error: Box::new(error),
    };
    let response = reqwest::Client::new()
        .get(&download_url)
        .send()
        .await
        .map_err(handle_error)?;

    if !response.status().is_success() {
        return Err(ProtoInstallerError::DownloadNotAvailable {
            version: version.to_owned(),
            status: Box::new(response.status()),
        }
        .into());
    }

    let total_size = response.content_length().unwrap_or(0);

    on_chunk(0, total_size);

    // Download in chunks
    let archive_file = temp_dir.as_ref().join(&download_file);
    let mut file = fs::create_file(&archive_file)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(handle_error)?;

        file.write_all(&chunk).map_err(|error| FsError::Write {
            path: archive_file.to_path_buf(),
            error: Box::new(error),
        })?;

        downloaded = cmp::min(downloaded + (chunk.len() as u64), total_size);

        on_chunk(downloaded, total_size);
    }

    Ok(DownloadResult {
        archive_file,
        file: download_file,
        file_stem: target_file,
        url: download_url,
    })
}

#[instrument]
pub fn install_release(
    download: DownloadResult,
    install_dir: impl AsRef<Path> + Debug,
    relocate_dir: impl AsRef<Path> + Debug,
    relocate_current: bool,
) -> miette::Result<bool> {
    let temp_dir = download
        .archive_file
        .parent()
        .unwrap()
        .join(&download.file_stem);
    let install_dir = install_dir.as_ref();
    let relocate_dir = relocate_dir.as_ref();
    let bin_names = if cfg!(windows) {
        vec!["proto.exe", "proto-shim.exe"]
    } else {
        vec!["proto", "proto-shim"]
    };

    trace!(
        source = ?download.archive_file,
        target = ?temp_dir,
        "Unpacking downloaded and installing proto release"
    );

    // Unpack the downloaded file
    Archiver::new(&temp_dir, &download.archive_file).unpack_from_ext()?;

    // Move the new binary to the install directory
    let mut installed = false;

    trace!(install_dir = ?install_dir, "Moving unpacked proto binaries to the install directory");

    let input_dirs = vec![temp_dir.join(&download.file_stem), temp_dir.clone()];
    let mut output_dirs = vec![install_dir.to_path_buf()];

    if relocate_current {
        if let Ok(current) = env::current_exe() {
            let current_dir = current.parent().unwrap();

            if current_dir != install_dir {
                output_dirs.push(current_dir.to_path_buf());
            }
        }
    }

    for bin_name in &bin_names {
        for input_dir in &input_dirs {
            let input_path = input_dir.join(bin_name);

            if !input_path.exists() {
                continue;
            }

            for output_dir in &output_dirs {
                let output_path = output_dir.join(bin_name);
                let relocate_path = relocate_dir.join(bin_name);

                if output_path.exists() {
                    self_replace(&output_path, &input_path, &relocate_path)?;
                } else {
                    fs::copy_file(&input_path, &output_path)?;
                    fs::update_perms(&output_path, None)?;
                }

                installed = true;
            }
        }
    }

    fs::remove(temp_dir)?;
    fs::remove(download.archive_file)?;

    // Track last used so operations like clean continue to work
    // correctly, otherwise we get into a weird state!
    if installed && relocate_dir.exists() {
        fs::write_file(
            relocate_dir.join(".last-used"),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
                .to_string(),
        )?;
    }

    Ok(installed)
}
