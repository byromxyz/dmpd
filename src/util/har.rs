use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{fs::File, path::Path};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
struct Har {
    log: Log,
}

#[derive(Serialize, Deserialize, Debug)]
struct Log {
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    response: Response,
    request: Request,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    content: Content,
    headers: Vec<Header>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Header {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    #[serde(rename = "mimeType")]
    mime_type: String,
    text: Option<String>,
    encoding: Option<String>,
}

pub fn extract_mpd(har_path_string: &String, output_dir_path: &PathBuf) -> Vec<PathBuf> {
    let path = std::path::Path::new(har_path_string);

    let mut paths: Vec<PathBuf> = vec![];

    let har_str = std::fs::read_to_string(path).expect(&format!("Cannot read HAR file"));

    // Parse the HAR file
    let har: Har = serde_json::from_str(&har_str).expect("Invalid HAR file");

    for entry in har.log.entries {
        if entry.response.content.mime_type != "application/dash+xml" {
            continue;
        }

        if entry.response.content.encoding.as_deref() == Some("base64") {
            eprintln!("Entry needs Base64 decoding");
            continue;
        }

        let text = &entry
            .response
            .content
            .text
            .expect("Found dash+xml entry with no text");

        let full_url = &entry.request.url;

        let fallback_header = Header {
            name: "date".to_owned(),
            value: "Unknown Date".to_owned(),
        };

        let date = &entry
            .response
            .headers
            .iter()
            .find(|header| header.name == "date")
            .unwrap_or(&fallback_header)
            .value;

        let parsed_date = DateTime::parse_from_rfc2822(date).expect("Failed to parse date");

        let formatted_date = parsed_date.format("%Y-%m-%d-%H-%M-%S").to_string();

        // Parse the URL
        let url = Url::parse(full_url).expect("Failed to parse URL");

        // Extract the path segments
        let path_segments: Vec<&str> = url.path_segments().map(|c| c.collect()).unwrap_or_default();

        // Get the last segment which is the filename
        let filename = path_segments
            .last()
            .expect(&format!("Could not parse manifest filename {}", full_url));

        let filename = format!("{}-{}", formatted_date, filename);

        let path = output_dir_path.join(&filename);

        println!("Writing {}", path.display());

        let mut file = File::create(&path).expect("Error creating file");

        match file.write_all(text.as_bytes()) {
            Ok(_r) => {
                println!("Saved {}", filename);

                paths.push(path);
            }
            Err(e) => panic!("Unable to save {}: {:?}", filename, e),
        }
    }

    paths
}
