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
        Some(value) => format!("Unmapped content type: {value}"),
        None => format!("No content type provided"),
    }
}
