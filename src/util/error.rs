use std::fmt;

#[allow(dead_code)]
pub enum ParseError {
    PathNotExist(String),
    CannotReadFileExtension(String),
    CannotReadFileStem(String),
    UnexpectedFileExtension(String),
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
        format!("{:?}", self)
    }
}

impl std::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::PathNotExist(filename) => {
                write!(f, "The provided path does not exist: {filename}")
            }
            ParseError::CannotReadFileStem(filename) => {
                write!(f, "Unable to read the input file stem for file: {filename}")
            }
            ParseError::CannotReadFileExtension(filename) => {
                write!(
                    f,
                    "Unable to read the provided file's extension for file: {filename}"
                )
            }
            ParseError::UnexpectedFileExtension(filename) => {
                write!(
                    f,
                    "Unsupported file extension. Provide a har or mpd file. Filename {filename}"
                )
            }
            ParseError::CannotOpenManifestFile => {
                write!(f, "Unable to open the provided manifest.")
            }
            ParseError::CannotParsePeriodStartAsU64 => {
                write!(f, "Unable to parse period start in ms when casting to u64.")
            }
            ParseError::CannotParsePeriodDurationAsU64 => write!(
                f,
                "Unable to parse period duration in ms when casting to u64."
            ),
            ParseError::CannotParseManifestFile => {
                write!(f, "Unable to parse the provided manifest.")
            }
            // ParseError::MpdWithoutAvailabilityStartTime => write!(f,
            //     "No availabilityStartTime on manifest. VOD manifests not yet supported."),
            ParseError::AdaptationSetWithoutContentType => {
                write!(f,
                "Found AdaptationSetWithoutContentType without or with an unexpected contentType")
            }
            ParseError::CannotInferRepresentationMimeType => {
                write!(f, "Unable to infer the mimeType for a Representation")
            }
            ParseError::CannotInferRepresentationCodecs => {
                write!(f, "Unable to infer the codecs for a Representation")
            }
            ParseError::CannotInferRepresentationAudioSamplingRate => write!(
                f,
                "Unable to infer the audioSamplingRate for a Representation"
            ),
            ParseError::CannotInferRepresentationFrameRate => {
                write!(f, "Unable to infer the frameRate for a Representation")
            }
            ParseError::RepresentationWithoutWidth => write!(f, "No width on Representation"),
            ParseError::RepresentationWithoutHeight => write!(f, "No height on Representation"),
            ParseError::RepresentationWithoutBandwidth => {
                write!(f, "No bandwidth on Representation")
            }
            ParseError::UnmappedRepresentationContentType => {
                write!(f, "A contentType has no description mapping")
            }
            // ParseError::SegmentTemplateWithoutTimescale => write!(f, "No timescale on SegmentTemplate),
            // ParseError::SegmentTemplateWithoutSegmentTimeline => write!(f,
            //     "No SegmentTimeline within a SegmentTemplate. SegmentList is not supported."),
            // ParseError::CannotInferSegmentTemplate => write!(f,
            //     "Expecting SegmentTemplate defined on Period but none found."),
            // ParseError::CannotInferSegmentTemplateMediaUrl => write!(f,
            //     "Cannot infer media URL for an inherited SegmentTemplate found on the Period"),
            // ParseError::SegmentWithoutTime => write!(f,
            //     "No t attribute on <S> segment with $Time$ based media URL"),
            ParseError::SegmentTimelineWithoutSegments => {
                write!(f, "Unable to get first segment from a SegmentTimeline")
            } // _ => {
              //     write!(f, "Unmapped parse error {}", self)
              // }
        }
    }
}

pub enum DrawError {
    CannotCreateFont,
}

impl DrawError {
    pub fn describe(&self) -> String {
        format!("{:?}", self)
    }
}
impl std::fmt::Debug for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawError::CannotCreateFont => {
                write!(f, "Unable to create font")
            }
        }
    }
}
