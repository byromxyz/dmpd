mod expand;
mod png;

pub trait Expanded {
    fn start_ms(&self) -> u64;
    fn end_ms(&self) -> u64;
    // fn duration_seconds(&self) -> f64;
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpandedMpd {
    pub periods: Vec<ExpandedPeriod>,
}

impl Expanded for ExpandedMpd {
    fn start_ms(&self) -> u64 {
        self.periods
            .first()
            .expect("Manifest with no periods")
            .start_ms()
    }
    fn end_ms(&self) -> u64 {
        self.periods
            .last()
            .expect("Manifest with no periods")
            .end_ms()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpandedPeriod {
    pub adaptation_sets: Vec<ExpandedAdaptationSet>,
    pub period_start_ms: u64,
    pub period_duration_ms: Option<u64>,
    pub id: String,
}

impl ExpandedPeriod {
    pub fn gap_start(&self) -> u64 {
        let segments_start_ms = self
            .adaptation_sets
            .first()
            .expect("No adaptation sets")
            .start_ms();

        if segments_start_ms < self.start_ms() {
            return self.start_ms() - segments_start_ms;
        } else {
            return 0u64;
        }
    }

    pub fn gap_end(&self) -> u64 {
        let segments_start_ms = self
            .adaptation_sets
            .first()
            .expect("No adaptation sets")
            .start_ms();

        segments_start_ms.min(self.start_ms())
    }
}

impl Expanded for ExpandedPeriod {
    fn start_ms(&self) -> u64 {
        let segments_start_ms = self
            .adaptation_sets
            .first()
            .expect("No adaptation sets")
            .start_ms();

        segments_start_ms
    }

    fn end_ms(&self) -> u64 {
        let segments_end_ms = self
            .adaptation_sets
            .first()
            .expect("No adaptation sets")
            .end_ms();

        segments_end_ms
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpandedSegmentTimeline {
    pub segments: Vec<ExpandedSegmentTimelineSegment>,
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

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpandedSegmentTimelineSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub repeat_ms: u64,
    pub size: u64,
    pub presentation_time_offset: u64,
}
