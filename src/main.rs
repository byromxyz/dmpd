use std::{fs, path::PathBuf};

use clap::Parser;
use expanded::ExpandedMpd;
use util::har::extract_mpd;

mod util {
    pub mod debug;
    pub mod error;
    pub mod har;
}

mod expanded;

use crate::util::{debug, error::ParseError};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(value_parser, required = true)]
    filename: String,

    #[clap(short, long, action)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

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
