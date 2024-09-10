#[derive(Debug)]
pub enum ParseError {
    CannotReadFileExtension,
    CannotReadFileStem,
    UnexpectedFileExtension,
    CannotOpenManifestFile,
    CannotParseManifestFile,
    CannotParsePeriodStartAsU64,
    CannotParsePeriodDurationAsU64,
    // MpdWithoutAvailabilityStartTime,
    AdaptationSetWithoutContentType,
    CannotInferRepresentationMimeType,
    CannotInferRepresentationCodecs,
    CannotInferRepresentationAudioSamplingRate,
    CannotInferRepresentationFrameRate,
    RepresentationWithoutWidth,
    RepresentationWithoutHeight,
    RepresentationWithoutBandwidth,
    UnmappedRepresentationContentType,
    // SegmentTemplateWithoutTimescale,
    // SegmentTemplateWithoutSegmentTimeline,
    SegmentTimelineWithoutSegments,
    // CannotInferSegmentTemplate,
    // CannotInferSegmentTemplateMediaUrl,
    // SegmentWithoutTime,
}

impl ParseError {
    pub fn describe(&self) -> String {
        let description = match self {
            ParseError::CannotReadFileStem => "Unable to read the input file stem",
            ParseError::CannotReadFileExtension => "Unable to read the provided file's extension.",
            ParseError::UnexpectedFileExtension => {
                "Unsupported file extension. Provide a har or mpd file"
            }
            ParseError::CannotOpenManifestFile => "Unable to open the provided manifest.",
            ParseError::CannotParsePeriodStartAsU64 => {
                "Unable to parse period start in ms when casting to u64."
            }
            ParseError::CannotParsePeriodDurationAsU64 => {
                "Unable to parse period duration in ms when casting to u64."
            }
            ParseError::CannotParseManifestFile => "Unable to parse the provided manifest.",
            // ParseError::MpdWithoutAvailabilityStartTime => {
            //     "No availabilityStartTime on manifest. VOD manifests not yet supported."
            // }
            ParseError::AdaptationSetWithoutContentType => {
                "Found AdaptationSetWithoutContentType without or with an unexpected contentType"
            }
            ParseError::CannotInferRepresentationMimeType => {
                "Unable to infer the mimeType for a Representation"
            }
            ParseError::CannotInferRepresentationCodecs => {
                "Unable to infer the codecs for a Representation"
            }
            ParseError::CannotInferRepresentationAudioSamplingRate => {
                "Unable to infer the audioSamplingRate for a Representation"
            }
            ParseError::CannotInferRepresentationFrameRate => {
                "Unable to infer the frameRate for a Representation"
            }
            ParseError::RepresentationWithoutWidth => "No width on Representation",
            ParseError::RepresentationWithoutHeight => "No height on Representation",
            ParseError::RepresentationWithoutBandwidth => "No bandwidth on Representation",
            ParseError::UnmappedRepresentationContentType => {
                "A contentType has no description mapping"
            }
            // ParseError::SegmentTemplateWithoutTimescale => "No timescale on SegmentTemplate",
            // ParseError::SegmentTemplateWithoutSegmentTimeline => {
            //     "No SegmentTimeline within a SegmentTemplate. SegmentList is not supported."
            // }
            // ParseError::CannotInferSegmentTemplate => {
            //     "Expecting SegmentTemplate defined on Period but none found."
            // }
            // ParseError::CannotInferSegmentTemplateMediaUrl => {
            //     "Cannot infer media URL for an inherited SegmentTemplate found on the Period"
            // }
            // ParseError::SegmentWithoutTime => {
            //     "No t attribute on <S> segment with $Time$ based media URL"
            // }
            ParseError::SegmentTimelineWithoutSegments => {
                "Unable to get first segment from a SegmentTimeline"
            }
        };

        format!("\nParseError::{:?}: {}\n", self, description)
    }
}

#[derive(Debug)]
pub enum DrawError {
    CannotCreateFont,
}

impl DrawError {
    pub fn describe(&self) -> String {
        let description = match self {
            DrawError::CannotCreateFont => "Unable to create font",
        };

        format!("\nDrawError::{:?}: {}\n", self, description)
    }
}
