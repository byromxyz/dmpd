use dash_mpd::{AdaptationSet, Representation};

use crate::util::error::ParseError;

pub fn describe_representation(
    representation: &Representation,
    adaptation_set: &AdaptationSet,
) -> String {
    let mime_type = representation
        .mimeType
        .as_ref()
        .or(adaptation_set.mimeType.as_ref())
        .expect(&ParseError::CannotInferRepresentationMimeType.describe());
    let codecs = representation
        .codecs
        .as_ref()
        .or(adaptation_set.codecs.as_ref())
        .expect(&ParseError::CannotInferRepresentationCodecs.describe());

    match &adaptation_set.contentType.as_deref() {
        Some("audio") => {
            let audio_sampling_rate = representation
                .audioSamplingRate
                .as_ref()
                .or(adaptation_set.audioSamplingRate.as_ref())
                .expect(&ParseError::CannotInferRepresentationAudioSamplingRate.describe());

            format!("{} {} {}Hz", mime_type, codecs, audio_sampling_rate)
        }
        Some("video") => {
            let frame_rate = representation
                .frameRate
                .as_ref()
                .or(adaptation_set.frameRate.as_ref());

            let width = representation.width;

            let height = representation.height;

            let bandwidth = representation.bandwidth;

            format!(
                "{:?} {:?} {:?}x{:?} {:?}fps {:?}bps",
                mime_type, codecs, width, height, frame_rate, bandwidth,
            )
        }
        _ => "Unknown contentType".to_owned(),
    }
}

pub fn parse_ms_duration(ms_total: u64) -> String {
    let ms = ms_total % 1000;

    let seconds_total = (ms_total - ms) / 1000;
    let seconds = seconds_total % 60;

    let minutes_total = (seconds_total - seconds) / 60;
    let minutes = minutes_total % 60;

    let hours_total = (minutes_total - minutes) / 60;

    if hours_total > 0 {
        format!(
            "{:02}hrs {:02}m {:02}.{:03}s",
            hours_total, minutes, seconds, ms
        )
    } else {
        format!("{:02}m {:02}.{:03}s", minutes, seconds, ms)
    }
}
