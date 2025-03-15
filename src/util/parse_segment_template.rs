use dash_mpd::SegmentTemplate;

use crate::expanded::{
    ExpandedSegmentDescription, ExpandedSegmentTimeline, ExpandedSegments, SegmentDescriptionType,
};

enum MediaTemplateType {
    TimeBased,
    NumberBased,
}

pub struct Context {
    pub period_duration_ms: Option<u64>,
}

fn resolve_inherited<'a, T>(
    templates: [&'a Option<SegmentTemplate>; 3],
    extract: impl Fn(&'a SegmentTemplate) -> Option<T>,
) -> Option<T> {
    templates
        .iter()
        .filter_map(|opt| opt.as_ref().and_then(&extract))
        .next()
}

pub fn parse_segment_template(
    representation_segment_template: &Option<SegmentTemplate>,
    adaptation_segment_template: &Option<SegmentTemplate>,
    period_segment_template: &Option<SegmentTemplate>,
    context: Context,
) -> ExpandedSegments {
    let segments = _parse_segment_template(
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
        context,
    );

    let _template: ExpandedSegments = ExpandedSegments::SegmentTemplate {
        segment_timeline: ExpandedSegmentTimeline { segments },
    };

    return _template;
}

fn _parse_segment_template(
    representation_segment_template: &Option<SegmentTemplate>,
    adaptation_segment_template: &Option<SegmentTemplate>,
    period_segment_template: &Option<SegmentTemplate>,
    state: Context,
) -> Vec<ExpandedSegmentDescription> {
    let templates = [
        representation_segment_template,
        adaptation_segment_template,
        period_segment_template,
    ];

    let timescale = resolve_inherited(templates, |st| st.timescale);
    let duration = resolve_inherited(templates, |st| st.duration);
    // let initialization = resolve_inherited(templates, |st| st.initialization);
    let media = resolve_inherited(templates, |st| st.media.as_ref());
    let start_number = resolve_inherited(templates, |st| st.startNumber.as_ref());
    let presentation_time_offset = resolve_inherited(templates, |st| st.presentationTimeOffset);
    let segment_timeline = resolve_inherited(templates, |st| st.SegmentTimeline.as_ref());

    let media = media.expect("No Media available"); // TODO - Add support
                                                    // let initialization = initialization.expect("No initialization available"); // TODO - Add support
    let timescale = timescale.expect("No timescale available");

    let template_type = if media.contains("$Time$") {
        MediaTemplateType::TimeBased
    } else if media.contains("$Number$") {
        MediaTemplateType::NumberBased
    } else {
        panic!("Media is neither Time nor Number based");
    };

    let mut _segments: Vec<ExpandedSegmentDescription> = vec![];

    let Some(segment_timeline) = segment_timeline else {
        // No SegmentTimeline found. Assume SegmentTemplate is fully templated.
        let period_duration_ms = state
            .period_duration_ms
            .expect("Fully templated SegmentTemplate but cannot infer duration");

        let duration =
            duration.expect("Expected duraiton for a fully templated SegmentTemplate") as u64;
        let start_number = *start_number.unwrap_or(&1);

        let segment_duration_ms = (duration as u128 * 1_000u128) / timescale as u128;

        let segment_count = (period_duration_ms as u128 / segment_duration_ms) as u64;

        // Though there is no SegmentTimeline, a fully templated SegmentTemplate could be represented
        // with a single <S> element inside a SegmentTimeline. Model the segments as such.

        let segments: Vec<ExpandedSegmentDescription> = vec![ExpandedSegmentDescription {
            start_ms: 0,
            end_ms: segment_count * segment_duration_ms as u64,
            duration_ms: segment_count * segment_duration_ms as u64,
            segment_duration_ms: segment_duration_ms as u64,
            segment_count: segment_count,
            // TODO - Could this be time based?
            description_type: SegmentDescriptionType::NumberTemplate {
                start_number,
                segment_count,
            },
        }];

        return segments;
    };

    let segments: Vec<ExpandedSegmentDescription> = segment_timeline
        .segments
        .iter()
        .scan(0u64, |running_timescale_unit, segment| {
            let start_timescale_unit = segment.t.unwrap_or(running_timescale_unit.clone());

            let relative_timescale_unit =
                start_timescale_unit - presentation_time_offset.unwrap_or(0);

            let start_ms = (relative_timescale_unit as u128 * 1_000u128 / timescale as u128) as u64;

            let segment_duration_ms = (segment.d as u128 * 1_000u128 / timescale as u128) as u64;

            let segment_count: u64 = 1 + segment.r.unwrap_or(0) as u64;

            *running_timescale_unit = start_timescale_unit + segment.d * segment_count;

            let description_type = match template_type {
                MediaTemplateType::NumberBased => SegmentDescriptionType::NumberTemplate {
                    start_number: *start_number.unwrap_or(&1),
                    segment_count,
                },
                MediaTemplateType::TimeBased => SegmentDescriptionType::TimeTemplate {
                    start_units: start_timescale_unit,
                    duration_units: segment.d,
                    segment_count,
                },
            };

            Some(ExpandedSegmentDescription {
                start_ms,
                end_ms: start_ms + segment_duration_ms * segment_count,
                duration_ms: segment_duration_ms * segment_count,
                segment_duration_ms,
                segment_count,
                description_type,
            })
        })
        .collect();

    return segments;
}

#[cfg(test)]
mod tests {
    use quick_xml::de::from_str;

    use super::*;

    fn parse(xml: &str) -> Option<SegmentTemplate> {
        let rtn: dash_mpd::SegmentTemplate = from_str(xml).unwrap();

        Some(rtn)
    }

    #[test]
    fn templated_basic() {
        let one = parse(
            r#"
            <SegmentTemplate
                media="seg_$Number$.m4s"
                initialization="init.mp4"
                duration="2000"
                timescale="1000"
                startNumber="1"
            />
        "#,
        );

        let result = _parse_segment_template(
            &one,
            &None,
            &None,
            Context {
                period_duration_ms: Some(6000u64),
            },
        );

        assert_eq!(result.len(), 1);

        // dash_mpd::par

        let first = result.first().unwrap();

        assert_eq!(first.start_ms, 0);
        assert_eq!(first.end_ms, 6000);
        assert_eq!(first.segment_duration_ms, 2000);
        assert_eq!(first.segment_count, 3);
    }

    #[test]
    fn templated_inherited_adaptation() {
        let one = parse(
            r#"
            <SegmentTemplate
                media="seg_$Number$.m4s"
                initialization="init.mp4"
                duration="2000"
                startNumber="1"
            />
        "#,
        );

        let two = parse(
            r#"
            <SegmentTemplate
                media="seg_$Number$.m4s"
                initialization="init.mp4"
                timescale="1000"
                startNumber="1"
            />
        "#,
        );

        let result = _parse_segment_template(
            &one,
            &two,
            &None,
            Context {
                period_duration_ms: Some(6000u64),
            },
        );

        assert_eq!(result.len(), 1);

        let first = result.first().unwrap();

        assert_eq!(first.start_ms, 0);
        assert_eq!(first.end_ms, 6000);
        assert_eq!(first.segment_duration_ms, 2000);
        assert_eq!(first.segment_count, 3);
    }

    #[test]
    fn templated_inherited_period() {
        let one = parse(
            r#"
            <SegmentTemplate
                media="seg_$Number$.m4s"
                initialization="init.mp4"
                duration="2000"
                startNumber="1"
            />
        "#,
        );

        let two = parse(
            r#"
            <SegmentTemplate
                media="seg_$Number$.m4s"
                initialization="init.mp4"
                timescale="1000"
                startNumber="1"
            />
        "#,
        );

        let result = _parse_segment_template(
            &one,
            &None,
            &two,
            Context {
                period_duration_ms: Some(6000u64),
            },
        );

        assert_eq!(result.len(), 1);

        let first = result.first().unwrap();

        assert_eq!(first.start_ms, 0);
        assert_eq!(first.end_ms, 6000);
        assert_eq!(first.segment_duration_ms, 2000);
        assert_eq!(first.segment_count, 3);
    }

    #[test]
    fn timeline() {
        let one = parse(
            r#"
            <SegmentTemplate
                media="seg_$Time$.m4s"
                initialization="init.mp4"
                timescale="1000"
            >
                <SegmentTimeline>
                    <S t="10000" d="2000" r="3" />
                    <S d="3000" r="2" />
                </SegmentTimeline>
            </SegmentTemplate>
        "#,
        );

        let result = _parse_segment_template(
            &one,
            &None,
            &None,
            Context {
                period_duration_ms: None,
            },
        );

        assert_eq!(result.len(), 2);

        let first = result.get(0).unwrap();

        assert_eq!(first.start_ms, 10_000);
        assert_eq!(first.end_ms, 18_000);
        assert_eq!(first.segment_duration_ms, 2000);
        assert_eq!(first.segment_count, 4);

        let second = result.get(1).unwrap();

        assert_eq!(second.start_ms, 18_000);
        assert_eq!(second.end_ms, 27_000);
        assert_eq!(second.segment_duration_ms, 3000);
        assert_eq!(second.segment_count, 3);
    }

    #[test]
    fn timeline_varying_t() {
        let one = parse(
            r#"
            <SegmentTemplate
                media="seg_$Time$.m4s"
                initialization="init.mp4"
                timescale="1000"
            >
                <SegmentTimeline>
                    <S t="0" d="1000" />
                    <S t="1500" d="2000" />
                    <S t="4000" d="3000" />
                </SegmentTimeline>
            </SegmentTemplate>
        "#,
        );

        let result = _parse_segment_template(
            &one,
            &None,
            &None,
            Context {
                period_duration_ms: None,
            },
        );

        assert_eq!(result.len(), 3);

        let first = result.get(0).unwrap();

        assert_eq!(first.start_ms, 0);
        assert_eq!(first.end_ms, 1000);
        assert_eq!(first.segment_duration_ms, 1000);
        assert_eq!(first.segment_count, 1);

        let second = result.get(1).unwrap();

        assert_eq!(second.start_ms, 1500);
        assert_eq!(second.end_ms, 3500);
        assert_eq!(second.segment_duration_ms, 2000);
        assert_eq!(second.segment_count, 1);

        let third = result.get(2).unwrap();

        assert_eq!(third.start_ms, 4000);
        assert_eq!(third.end_ms, 7000);
        assert_eq!(third.segment_duration_ms, 3000);
        assert_eq!(third.segment_count, 1);
    }
}
