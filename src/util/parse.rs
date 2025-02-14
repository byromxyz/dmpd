use dash_mpd::{AdaptationSet, Period, Representation, SegmentTemplate};

use crate::expanded::{ExpandedSegmentTimeline, ExpandedSegmentTimelineSegment, ExpandedSegments};
use crate::util::error::ParseError;

use crate::debug;

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
                .or(adaptation_set.frameRate.as_ref())
                .expect(&ParseError::CannotInferRepresentationFrameRate.describe());

            let width = representation
                .width
                .expect(&ParseError::RepresentationWithoutWidth.describe());

            let height = representation
                .height
                .expect(&ParseError::RepresentationWithoutHeight.describe());

            let bandwidth = representation
                .bandwidth
                .expect(&ParseError::RepresentationWithoutBandwidth.describe());

            format!(
                "{} {} {}x{} {}fps {}bps",
                mime_type, codecs, width, height, frame_rate, bandwidth,
            )
        }
        _ => {
            panic!(
                "{}",
                &ParseError::UnmappedRepresentationContentType.describe()
            )
        }
    }
}

pub fn parse_period_start_ms(period: &Period, previous_period_end_ms: u64) -> u64 {
    match period.start {
        Some(duration) => {
            let start_ms: u64 = duration
                .as_millis()
                .try_into()
                .expect(&ParseError::CannotParsePeriodStartAsU64.describe());

            let gap: i64 = match start_ms.cmp(&previous_period_end_ms) {
                std::cmp::Ordering::Greater => (start_ms - previous_period_end_ms) as i64,
                std::cmp::Ordering::Less => -1 * (previous_period_end_ms - start_ms) as i64,
                std::cmp::Ordering::Equal => 0,
            };

            debug!(
                "  Start time {}ms. {}ms gap to the previous period.",
                start_ms, gap
            );

            start_ms
        }
        None => {
            debug!(
                "No start time defined, using the end time of the previous period (or 0), {}",
                previous_period_end_ms
            );

            previous_period_end_ms
        }
    }
}

pub fn parse_period_duration_ms(period: &Period) -> Option<u64> {
    match period.duration {
        Some(duration) => {
            let duration_ms: u64 = duration
                .as_millis()
                .try_into()
                .expect(&ParseError::CannotParsePeriodDurationAsU64.describe());
            debug!("  Duration {}ms.", duration_ms);

            Some(duration_ms)
        }
        None => {
            debug!("No duration defined. Period ends naturally when its segments end (?).",);

            None
        }
    }
}

pub fn parse_segment_template(
    representation_segment_template: &Option<SegmentTemplate>,
    adaptation_segment_template: &Option<SegmentTemplate>,
    period_segment_template: &Option<SegmentTemplate>,
    period_start_ms: u64,
) -> ExpandedSegments {
    let timescale = [
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
    ]
    .iter()
    .filter_map(|opt| opt.as_ref().and_then(|st| st.timescale))
    .next()
    .expect("No timescale available");

    let timeline = [
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
    ]
    .iter()
    .filter_map(|opt| opt.as_ref().and_then(|st| st.SegmentTimeline.as_ref()))
    .next()
    .expect("No SegmentTimeline available");

    let media = [
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
    ]
    .iter()
    .filter_map(|opt| opt.as_ref().and_then(|st| st.media.as_ref()))
    .next()
    .expect("No SegmentTemplate media available");

    let presentation_time_offset = [
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
    ]
    .iter()
    .filter_map(|opt| opt.as_ref().and_then(|st| st.presentationTimeOffset))
    .next()
    .unwrap_or(0);

    if media.contains("$Time$") {
        debug!("  Media template contains $Time$ placeholder");
    } else if media.contains("$Number$") {
        debug!("  Media template contains $Number$ placeholder");
    }

    let mut _segments: Vec<ExpandedSegmentTimelineSegment> = vec![];

    let mut running_time_unit: u64 = timeline
        .segments
        .first()
        .expect(&ParseError::SegmentTimelineWithoutSegments.describe())
        .t
        .unwrap_or(0);

    for s in timeline.segments.iter() {
        let segment_repeat = match s.r {
            Some(r) => r as u64 + 1,
            None => 1u64,
        };

        let segment_duration_ticks = s.d as u64;
        let segment_duration_ms = segment_duration_ticks * 1000 / timescale;

        let segment_t = match s.t {
            Some(t) => t,
            None => running_time_unit,
        };

        running_time_unit = segment_t + segment_duration_ticks * segment_repeat;

        //  let start_ticks = segment_t - presentation_time_offset;

        let segment_element_start_ms =
            period_start_ms + 1000 * (segment_t - presentation_time_offset) / timescale;

        let segment_element_duration_ms = segment_duration_ms * segment_repeat;

        let segment_element_end_ms = segment_element_start_ms + segment_element_duration_ms;

        debug!(
            "  <S> t={} ({}ms), d={} ({}ms). {} segments, ending at {}ms -- {} {}",
            segment_t,
            segment_element_start_ms,
            segment_duration_ticks,
            segment_duration_ms,
            segment_repeat,
            segment_element_end_ms,
            presentation_time_offset,
            timescale
        );

        let segment = ExpandedSegmentTimelineSegment {
            start_ms: segment_element_start_ms,
            duration_ms: segment_element_duration_ms,
            end_ms: segment_element_end_ms,
            segment_duration_ms,
            segment_count: segment_repeat,
            presentation_time_offset: presentation_time_offset,
        };

        _segments.push(segment);
    }

    // The SegmentTimeline element shall contain a list of S elements each of which describes a sequence of contiguous
    // segments of identical MPD duration. The S element contains a mandatory @d attribute specifying the MPD duration,
    // an optional @r repeat count attribute specifying the number of contiguous Segments with identical MPD duration
    // minus one and an optional @t time attribute. The value of the @t attribute minus the value of the
    // @presentationTimeOffset specifies the MPD start time of the first Segment in the series.

    let _template = ExpandedSegments::SegmentTemplate {
        segment_timeline: ExpandedSegmentTimeline {
            segments: _segments,
        },
    };

    return _template;
}
