use clap::{App, Parser};
use std::{fmt, fs, path::PathBuf, sync::Arc, thread, u64};

use expanded::ExpandedMpd;
use util::har::extract_mpd;

mod util {
    pub mod debug;
    pub mod draw;
    pub mod error;
    pub mod har;
    pub mod parse;
    pub mod parse_segment_template;
    pub mod update;
}

mod expanded;

use crate::util::{debug, error::ParseError, update};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(value_parser, required = true)]
    filename: String,

    #[clap(short, long, action)]
    debug: bool,

    #[clap(long)]
    slice: bool,

    #[clap(short, long, action)]
    plan: bool,

    #[clap(long, default_value = "120000", value_parser = parse_max_duration)]
    max_duration_ms: u64,

    #[clap(long, default_value_t = 120)]
    image_padding_x: u32,

    #[clap(long, default_value_t = 90)]
    image_padding_y: u32,

    #[clap(long, default_value_t = 10)]
    period_title_x_spacing: u32,

    #[clap(long, default_value_t = 40)]
    scale: u32,

    #[clap(long, default_value_t = 20.0)]
    font_size: f32,

    #[clap(long, default_value_t = 20)]
    adaptation_set_padding: u32,

    #[clap(long, default_value_t = 40)]
    representation_width: u32,

    #[clap(long, default_value_t = 5)]
    representation_padding: u32,

    #[clap(long)]
    from_ms: Option<i64>,

    #[clap(long)]
    to_ms: Option<i64>,
}

fn parse_max_duration(s: &str) -> Result<u64, String> {
    let val: i64 = s.parse().map_err(|e| format!("Invalid timeout: {}", e))?;

    if val == -1 {
        Ok(u64::MAX)
    } else if val > 0 {
        Ok(val as u64)
    } else {
        Err("Timeout must be -1 or a positive integer".to_string())
    }
}

struct Config {
    max_duration_ms: u64,
    image_padding_x: u32,
    image_padding_y: u32,
    period_title_x_spacing: u32,
    scale: u32,
    slice: bool,
    font_size: f32,
    adaptation_set_padding: u32,
    representation_width: u32,
    representation_padding: u32,
    from_ms: Option<u64>,
    to_ms: Option<u64>,
}

enum AppError {
    ParseError(ParseError),
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ParseError(err) => write!(f, "{:?}", err),
        }
    }
}

fn main() -> Result<(), AppError> {
    let args: Args = Args::parse();

    update::check_updates();

    debug::DEBUG.store(args.debug, std::sync::atomic::Ordering::Relaxed);

    let path = std::path::Path::new(&args.filename);

    if !path.exists() {
        return Err(AppError::ParseError(ParseError::PathNotExist(
            args.filename.clone(),
        )));
    }

    if path.is_file() {
        let extension = std::path::Path::new(&args.filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or(AppError::ParseError(ParseError::CannotReadFileExtension(
                args.filename.clone(),
            )))?;

        let file_stem = std::path::Path::new(&args.filename)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or(AppError::ParseError(ParseError::CannotReadFileStem(
                args.filename.clone(),
            )))?;

        match extension {
            "mpd" => {
                handle_mpd(&args.filename, &args)?;
            }
            "har" => {
                handle_har(file_stem, &args)?;
            }
            _ => {
                return Err(AppError::ParseError(ParseError::UnexpectedFileExtension(
                    args.filename,
                )))
            }
        }
    } else {
        handle_dir(&path, &args)?;
    }

    Ok(())
}

fn handle_mpd(filename: &str, args: &Args) -> Result<(), AppError> {
    let xml =
        std::fs::read_to_string(&filename).expect(&ParseError::CannotOpenManifestFile.describe());

    let mpd: dash_mpd::MPD =
        dash_mpd::parse(&xml).expect(&ParseError::CannotParseManifestFile.describe());

    let expanded = ExpandedMpd::new(mpd);

    let from_ms = args.from_ms.map_or_else(
        || expanded.start_timestamp_ms(),
        |val| {
            if val < 0 {
                expanded.end_timestamp_ms() - val.abs() as u64
            } else {
                val as u64
            }
        },
    );

    let to_ms = args.to_ms.map_or_else(
        || expanded.end_timestamp_ms(),
        |val| {
            if val < 0 {
                expanded.end_timestamp_ms() - val.abs() as u64
            } else {
                val as u64
            }
        },
    );

    let config = Config {
        max_duration_ms: args.max_duration_ms,
        image_padding_x: args.image_padding_x,
        image_padding_y: args.image_padding_y,
        period_title_x_spacing: args.period_title_x_spacing,
        scale: args.scale,
        slice: args.slice,
        font_size: args.font_size,
        adaptation_set_padding: args.adaptation_set_padding,
        representation_width: args.representation_width,
        representation_padding: args.representation_padding,
        from_ms: Some(from_ms),
        to_ms: Some(to_ms),
    };

    let duration_ms = to_ms - from_ms;

    if args.plan {
        if config.slice && duration_ms > config.max_duration_ms {
            for timestamp in (from_ms..=to_ms).step_by(
                config
                    .max_duration_ms
                    .try_into()
                    .expect("Unable to convert step size to usize"),
            ) {
                let new_config = Config {
                    from_ms: Some(timestamp),
                    to_ms: Some((timestamp + config.max_duration_ms)),
                    ..config
                };

                let json_plan = expanded.to_plan(&new_config);

                let json_path = args
                    .filename
                    .replace(".mpd", &format!("-{}.json", timestamp));

                std::fs::write(&json_path, json_plan).expect("Failed to write plan to JSON file");
            }
        } else {
            let json_plan = expanded.to_plan(&config);

            let json_path = args.filename.replace(".mpd", ".plan.json");

            std::fs::write(&json_path, json_plan).expect("Failed to write plan to JSON file");
        }
    } else {
        if config.slice && duration_ms > config.max_duration_ms {
            let expanded = Arc::new(expanded);

            let mut handles = vec![];

            for timestamp in (from_ms..=to_ms).step_by(
                config
                    .max_duration_ms
                    .try_into()
                    .expect("Unable to convert step size to usize"),
            ) {
                let expanded = Arc::clone(&expanded);
                let filename = args.filename.clone();
                let handle = thread::spawn(move || {
                    let new_config = Config {
                        from_ms: Some(timestamp),
                        to_ms: Some(timestamp + config.max_duration_ms),
                        ..config
                    };

                    if let Some(image) = expanded.to_png(&new_config) {
                        image
                            .save(filename.replace(".mpd", &format!("-{}.png", timestamp)))
                            .unwrap();
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().expect("Thread panicked");
            }

            println!("Threads done");
        } else {
            if let Some(image) = expanded.to_png(&config) {
                image.save(args.filename.replace(".mpd", ".png")).unwrap();
            }
        }
    }

    Ok(())
}

fn handle_har(file_stem: &str, args: &Args) -> Result<(), AppError> {
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

        handle_mpd(path_str, &args)?;
    }

    Ok(())
}

fn handle_dir(path: &std::path::Path, args: &Args) -> Result<(), AppError> {
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
        let create_path_result = fs::create_dir(&png_path);

        if create_path_result.is_err() {
            panic!("Unable to create path {:?}", create_path_result.err())
        }
    }

    for filename in file_names {
        let path_str = filename
            .to_str()
            .expect("Unable to convert path to filename");

        handle_mpd(&path_str, &args)?;
    }

    Ok(())
}
