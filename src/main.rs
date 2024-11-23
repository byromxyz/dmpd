use std::{fs, path::PathBuf};

use ::serde::Deserialize;
use clap::Parser;
use expanded::ExpandedMpd;
use reqwest::blocking::Client;
use semver::Version;
use serde_json::from_str;
use util::har::extract_mpd;

mod util {
    pub mod debug;
    pub mod error;
    pub mod har;
}

mod expanded;

use crate::util::{debug, error::ParseError};

#[derive(Deserialize, Debug)]
pub struct Release {
    pub url: String,
    pub assets_url: String,
    pub upload_url: String,
    pub html_url: String,
    pub id: u64,
    pub author: Author,
    pub node_id: String,
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub published_at: String,
    pub assets: Vec<Asset>,
    pub tarball_url: String,
    pub zipball_url: String,
    pub body: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Author {
    pub login: String,
    pub id: u64,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    pub user_view_type: Option<String>,
    #[serde(rename = "type")]
    pub user_type: String,
    pub site_admin: bool,
}

#[derive(Deserialize, Debug)]
pub struct Asset {
    pub url: String,
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub label: Option<String>,
    pub uploader: Author,
    pub content_type: String,
    pub state: String,
    pub size: u64,
    pub download_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub browser_download_url: String,
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(value_parser, required = true)]
    filename: String,

    #[clap(short, long, action)]
    debug: bool,
}

/// Compare the current binary version against a provided version
///
/// Returns:
/// - `-1` if the current version is less than the provided version.
/// - `0` if the versions are equal.
/// - `1` if the current version is greater than the provided version.
fn compare_versions(current_version: &str, provided_version: &str) -> Result<i8, String> {
    let current = Version::parse(current_version)
        .map_err(|err| format!("Failed to parse current version: {}", err))?;
    let provided = Version::parse(provided_version)
        .map_err(|err| format!("Failed to parse provided version: {}", err))?;

    Ok(if current < provided {
        -1
    } else if current == provided {
        0
    } else {
        1
    })
}

fn check_updates() {
    let client = Client::new();

    let http_result = client
        .get("https://api.github.com/repos/byromxyz/dmpd/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "dmpd")
        .send()
        .unwrap();

    if !http_result.status().is_success() {
        println!(
            "Unable to lookup latest release version. Skipping update check. {:?}",
            http_result.status()
        );
        return;
    }

    let body = from_str::<Release>(&http_result.text().unwrap()).unwrap();

    let version = &body.tag_name;

    // Current version of the binary (from Cargo.toml)
    let current_version = env!("CARGO_PKG_VERSION");

    if compare_versions(current_version, version) >= Ok(0) {
        return;
    }

    println!(
        "A newer version {} is available.\nSee {}",
        version, &body.html_url
    );
}

fn main() {
    let args = Args::parse();

    check_updates();

    debug::DEBUG.store(args.debug, std::sync::atomic::Ordering::Relaxed);

    let path = std::path::Path::new(&args.filename);

    debug!("Input: {:?}", args);

    if path.is_dir() {
        let file_names: Vec<PathBuf> = fs::read_dir(path)
            .unwrap() // :')
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter_map(|entry| match entry.extension() {
                Some(ext) => {
                    if ext.eq_ignore_ascii_case("mpd") {
                        Some(entry)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        // Store all images in a png folder
        let png_path = path.join("png");

        if !png_path.exists() {
            debug!("Creating path {:?}", png_path);
            let create_path_result = fs::create_dir(&png_path);

            if create_path_result.is_err() {
                panic!("Unable to create path {:?}", create_path_result.err())
            }
        }

        for filename in file_names {
            let xml = std::fs::read_to_string(&filename)
                .expect(&ParseError::CannotOpenManifestFile.describe());

            let mpd: dash_mpd::MPD =
                dash_mpd::parse(&xml).expect(&ParseError::CannotParseManifestFile.describe());

            let mut expanded = ExpandedMpd::new(mpd);

            if let Some(image) = expanded.to_png(args.debug) {
                let name: String = filename
                    .file_stem()
                    .expect("xx")
                    .to_str()
                    .expect("xx")
                    .to_owned();

                let output_path = png_path.join(format!("{}.png", name));

                image.save(output_path).unwrap();
            }
        }
    } else {
        let extension = std::path::Path::new(&args.filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .expect(&ParseError::CannotReadFileExtension.describe());

        let file_stem = std::path::Path::new(&args.filename)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .expect(&ParseError::CannotReadFileStem.describe());

        match extension {
            "mpd" => {
                let xml = std::fs::read_to_string(&args.filename)
                    .expect(&ParseError::CannotOpenManifestFile.describe());

                let mpd: dash_mpd::MPD =
                    dash_mpd::parse(&xml).expect(&ParseError::CannotParseManifestFile.describe());

                let mut expanded = ExpandedMpd::new(mpd);

                if let Some(image) = expanded.to_png(args.debug) {
                    image.save(args.filename.replace(".mpd", ".png")).unwrap();
                }
            }
            "har" => {
                let parent_path = std::path::Path::new(&args.filename)
                    .parent()
                    .expect(&format!(
                        "Unable to read parent dir for input {}",
                        &args.filename
                    ));

                let output_path = parent_path.join(file_stem);

                if !output_path.exists() {
                    let create_path_result = fs::create_dir(&output_path);

                    if create_path_result.is_err() {
                        panic!("Unable to create path {:?}", create_path_result.err())
                    }
                }

                let png_path = output_path.join("png");

                if !png_path.exists() {
                    let create_path_result = fs::create_dir(&png_path);

                    if create_path_result.is_err() {
                        panic!("Unable to create path {:?}", create_path_result.err())
                    }
                }

                let mpd_path = output_path.join("mpd");

                if !mpd_path.exists() {
                    let create_path_result = fs::create_dir(&mpd_path);

                    if create_path_result.is_err() {
                        panic!("Unable to create path {:?}", create_path_result.err())
                    }
                }

                let paths = extract_mpd(&args.filename, &mpd_path);

                for path in paths {
                    let path_str = path.to_str().expect("Unable to convert path to filename");

                    let xml = std::fs::read_to_string(path_str)
                        .expect(&ParseError::CannotOpenManifestFile.describe());

                    let mpd: dash_mpd::MPD = dash_mpd::parse(&xml)
                        .expect(&ParseError::CannotParseManifestFile.describe());

                    let mut expanded = ExpandedMpd::new(mpd);

                    if let Some(image) = expanded.to_png(args.debug) {
                        let file_stem = path
                            .file_stem()
                            .and_then(std::ffi::OsStr::to_str)
                            .expect(&format!("Unable to read file stem for path {:?}", path));

                        image
                            .save(png_path.join(format!("{}.png", file_stem)))
                            .expect(&format!("Unable to save png file for path {:?}", path));
                    }
                }
            }
            _ => panic!("{}", &ParseError::UnexpectedFileExtension.describe()),
        }
    }
}
