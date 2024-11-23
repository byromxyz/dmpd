use dash_mpd::{SegmentTemplate, MPD};

use crate::{debug, util::error::ParseError};

use super::{
    Expanded, ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation,
    ExpandedSegmentTimeline, ExpandedSegmentTimelineSegment, ExpandedSegments,
};

impl ExpandedMpd {
    pub fn new(mpd: MPD) -> Self {
        let mut _periods: Vec<ExpandedPeriod> = vec![];

        let mut previous_period_end_ms = 0u64;

        for p in mpd.periods {
            let period_id = p.id.clone().unwrap_or("No ID".to_owned());

            debug!("\nPeriod: {}", period_id);

            let period_start_ms = match p.start {
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
            };

            let period_duration_ms: Option<u64> = match p.duration {
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
            };

            debug!("  {} AdaptationSets", p.adaptations.len());

            let mut adaptation_sets: Vec<ExpandedAdaptationSet> = vec![];

            for adaptation in p.adaptations.iter() {
                let adaptation_set_id = adaptation.id.clone().unwrap_or("No ID".to_owned());

                debug!(
                    "\n  AdaptationSet {} ({}) has {} Representations",
                    adaptation_set_id,
                    adaptation
                        .contentType
                        .clone()
                        .unwrap_or("No contentType".to_owned()),
                    adaptation.representations.len()
                );

                let mut representations: Vec<ExpandedRepresentation> = vec![];

                let base_mime_type = &adaptation.mimeType;
                let base_content_type = match adaptation.contentType {
                    Some(ref s) if s == "audio" => "audio",
                    Some(ref s) if s == "video" => "video",
                    _ => panic!(
                        "{}",
                        &ParseError::AdaptationSetWithoutContentType.describe()
                    ),
                };
                let base_codecs = &adaptation.codecs;
                let base_frame_rate = &adaptation.frameRate;
                let base_audio_sampling_rate = &adaptation.audioSamplingRate;

                for rep in adaptation.representations.iter() {
                    let representation_id = rep.id.clone().unwrap_or("No ID".to_owned());

                    let representation_description = match &base_content_type as &str {
                        "audio" => {
                            format!(
                                "{} {} {}Hz",
                                rep.mimeType.clone().unwrap_or_else(|| {
                                    base_mime_type.clone().expect(
                                        &ParseError::CannotInferRepresentationMimeType.describe(),
                                    )
                                }),
                                rep.codecs.clone().unwrap_or_else(|| {
                                    base_codecs.clone().expect(
                                        &ParseError::CannotInferRepresentationCodecs.describe(),
                                    )
                                }),
                                rep.audioSamplingRate.clone().unwrap_or_else(|| {
                                    base_audio_sampling_rate.clone().expect(
                                        &ParseError::CannotInferRepresentationAudioSamplingRate
                                            .describe(),
                                    )
                                }),
                            )
                        }
                        "video" => {
                            format!(
                                "{} {} {}x{} {}fps {}bps",
                                rep.mimeType.clone().unwrap_or_else(|| {
                                    base_mime_type.clone().expect(
                                        &ParseError::CannotInferRepresentationMimeType.describe(),
                                    )
                                }),
                                rep.codecs.clone().unwrap_or_else(|| {
                                    base_codecs.clone().expect(
                                        &ParseError::CannotInferRepresentationCodecs.describe(),
                                    )
                                }),
                                rep.width
                                    .expect(&ParseError::RepresentationWithoutWidth.describe()),
                                rep.height
                                    .expect(&ParseError::RepresentationWithoutHeight.describe()),
                                rep.frameRate.clone().unwrap_or_else(|| {
                                    base_frame_rate.clone().expect(
                                        &ParseError::CannotInferRepresentationFrameRate.describe(),
                                    )
                                }),
                                rep.bandwidth
                                    .expect(&ParseError::RepresentationWithoutBandwidth.describe()),
                            )
                        }
                        _ => {
                            panic!(
                                "{}",
                                &ParseError::UnmappedRepresentationContentType.describe()
                            )
                        }
                    };

                    debug!(
                        "\n  Representation {}: {}",
                        representation_id, representation_description
                    );

                    let segments = parse_segment_template(
                        &rep.SegmentTemplate,
                        &adaptation.SegmentTemplate,
                        &p.SegmentTemplate,
                        period_start_ms,
                    );

                    representations.push(ExpandedRepresentation { segments });
                }

                let adaptation_set = ExpandedAdaptationSet {
                    representations,
                    content_type: base_content_type.to_owned(),
                };

                adaptation_sets.push(adaptation_set);
            }
            let period = ExpandedPeriod {
                period_start_ms,
                period_duration_ms,
                adaptation_sets,
                id: period_id,
            };

            previous_period_end_ms = period.end_ms();

            _periods.push(period);
        }

        ExpandedMpd { periods: _periods }
    }
}

fn parse_segment_template(
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
