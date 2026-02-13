use std::time::Duration;

use chrono::DateTime;

mod expand;
mod png;

/// Type representing an xs:dateTime, as per <https://www.w3.org/TR/xmlschema-2/#dateTime>
// Something like 2021-06-03T13:00:00Z or 2022-12-06T22:27:53
pub type XsDatetime = DateTime<chrono::offset::Utc>;

pub trait Expanded {
    fn start_ms(&self) -> u64;
    fn end_ms(&self) -> u64;
    // fn duration_seconds(&self) -> f64;
}

#[derive(Debug)]
pub enum MpdType {
    Dynamic {
        availability_start_time: XsDatetime,
        publish_time: XsDatetime,
        minimum_update_period: Duration,
        suggested_presentation_delay: Option<Duration>,
    },
    Static {
        media_presentation_duration: Option<Duration>,
    },
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpandedMpd {
    pub periods: Vec<ExpandedPeriod>,
    pub mpd_type: MpdType,
}

// TODO - Reconsider naming of start_ms, end_ms, etc.

impl ExpandedMpd {
    pub fn start_timestamp_ms(&self) -> u64 {
        let first_period = self.periods.first().expect("No periods");

        let start_timestamp = first_period.mpd_start_ms + first_period.start_ms();

        start_timestamp
    }

    pub fn end_timestamp_ms(&self) -> u64 {
        match self.mpd_type {
            MpdType::Dynamic {
                availability_start_time,
                publish_time,
                minimum_update_period,
                suggested_presentation_delay: _,
            } => {
                // Calculate the latest possible "now" based on the publish time and update period
                let now = publish_time + minimum_update_period;

                // Calculate a maximum duration using availability_start_time as the zero-point
                let valid_timeline_duration =
                    (now - availability_start_time).num_milliseconds() as u64;

                let last_period = self.periods.last().expect("No periods.");

                let internal_timeline_duration =
                    last_period.mpd_start_ms + last_period.period_duration_ms;

                return valid_timeline_duration.max(internal_timeline_duration);
            }
            MpdType::Static {
                media_presentation_duration,
            } => {
                return media_presentation_duration
                    .expect("Static manifest without mediaPresentationDuration ??")
                    .as_millis() as u64;
            }
        }
    }
}

impl Expanded for ExpandedMpd {
    fn start_ms(&self) -> u64 {
        let first_period = self.periods.first().expect("Manifest with no periods");

        first_period.mpd_start_ms + first_period.start_ms()
    }
    fn end_ms(&self) -> u64 {
        let last_period = self.periods.last().expect("Manifest with no periods");

        last_period.mpd_start_ms + last_period.end_ms()
    }
}

#[derive(Debug, Clone)]
pub struct ExpandedEvent {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub id: String,
    pub scheme_id_uri: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpandedPeriod {
    pub adaptation_sets: Vec<ExpandedAdaptationSet>,
    pub mpd_start_ms: u64,
    pub mpd_end_ms: u64,
    pub period_duration_ms: u64,
    pub id: String,
    pub events: Vec<ExpandedEvent>,
}

impl Expanded for ExpandedPeriod {
    fn start_ms(&self) -> u64 {
        self.adaptation_sets
            .iter()
            .map(|adaptation_set| adaptation_set.start_ms())
            .min()
            .expect("Could not find segments start time")
    }

    fn end_ms(&self) -> u64 {
        self.adaptation_sets
            .iter()
            .map(|adaptation_set| adaptation_set.end_ms())
            .max()
            .expect("Could not find segments start time")
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpandedAdaptationSet {
    pub content_type: String, // audio or video
    pub representations: Vec<ExpandedRepresentation>,
}

impl Expanded for ExpandedAdaptationSet {
    fn start_ms(&self) -> u64 {
        self.representations
            .first()
            .expect("AdaptationSet with no representations")
            .start_ms()
    }
    fn end_ms(&self) -> u64 {
        self.representations
            .last()
            .expect("AdaptationSet with no representations")
            .end_ms()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpandedRepresentation {
    pub segments: ExpandedSegments,
}

impl Expanded for ExpandedRepresentation {
    fn start_ms(&self) -> u64 {
        self.segments.start_ms()
    }
    fn end_ms(&self) -> u64 {
        self.segments.end_ms()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ExpandedSegments {
    SegmentTemplate {
        segment_timeline: ExpandedSegmentTimeline,
    },
    SegmentList, // TODO
}

impl Expanded for ExpandedSegments {
    fn start_ms(&self) -> u64 {
        match &self {
            ExpandedSegments::SegmentTemplate { segment_timeline } => segment_timeline.start_ms(),
            ExpandedSegments::SegmentList => 0u64,
        }
    }
    fn end_ms(&self) -> u64 {
        match &self {
            ExpandedSegments::SegmentTemplate { segment_timeline } => segment_timeline.end_ms(),
            ExpandedSegments::SegmentList => 0u64,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpandedSegmentTimeline {
    pub segments: Vec<ExpandedSegmentDescription>,
}

impl Expanded for ExpandedSegmentTimeline {
    fn start_ms(&self) -> u64 {
        self.segments
            .first()
            .expect("SegmentTimeline with no segments")
            .start_ms
    }
    fn end_ms(&self) -> u64 {
        self.segments
            .last()
            .expect("SegmentTimeline with no segments")
            .end_ms
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SegmentDescriptionType {
    NumberTemplate {
        start_number: u64,
        segment_count: u64,
    },
    TimeTemplate {
        start_units: u64,
        duration_units: u64,
        segment_count: u64,
    },
    Basic {
        url: String,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpandedSegmentDescription {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub segment_duration_ms: u64,
    pub segment_count: u64,
    pub description_type: SegmentDescriptionType,
}
