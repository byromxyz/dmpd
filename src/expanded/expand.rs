use chrono::Utc;
use dash_mpd::{Period, MPD};

use crate::{
    debug,
    util::{parse::describe_representation, parse_segment_template},
};

use super::{ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation};
use std::time::Duration;

fn get_period_start_ms(current: &Period, prev_end_ms: u64) -> u64 {
    current
        .start
        .map(|s| s.as_millis() as u64)
        .unwrap_or(prev_end_ms)
}

fn get_period_duration_ms(
    current: &Period,
    next: Option<&Period>,
    start_ms: u64,
    is_dynamic: bool,
    media_presentation_duration: Option<Duration>,
) -> u64 {
    if let Some(duration) = current.duration {
        return duration.as_millis() as u64;
    }

    if let Some(next) = next {
        if let Some(next_start) = next.start {
            return next_start.as_millis() as u64 - start_ms;
        }
    }

    // No duration defined and no next period.
    // If a dynamic manifest calculate until wall clock time.

    if is_dynamic {
        let now = Utc::now();

        return now.timestamp_millis() as u64 - start_ms;
    } else {
        let mpd_duration_ms = media_presentation_duration
            .expect("Static manifest without mediaPresentationDuration")
            .as_millis();

        return mpd_duration_ms as u64 - start_ms;
    }

    panic!("Unable to parse period duration");
}

impl ExpandedMpd {
    pub fn new(mpd: MPD) -> Self {
        let mut _periods: Vec<ExpandedPeriod> = vec![];

        let mut previous_period_end_ms = 0u64;

        for (i, p) in mpd.periods.iter().enumerate() {
            let next = mpd.periods.get(i + 1);

            let is_dynamic = mpd.mpdtype.as_deref() == Some("dynamic");

            let period_start_ms = get_period_start_ms(p, previous_period_end_ms);
            let period_duration_ms = get_period_duration_ms(
                p,
                next,
                period_start_ms,
                is_dynamic,
                mpd.mediaPresentationDuration,
            );
            let period_end_ms = period_start_ms + period_duration_ms;

            let period_id = p.id.clone().unwrap_or("No ID".to_owned());

            debug!("\nPeriod: {}", period_id);

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

                let content_type = match adaptation.contentType {
                    Some(ref s) if s == "audio" => "audio",
                    Some(ref s) if s == "video" => "video",
                    Some(ref s) if s == "text" => "text",
                    _ => match adaptation.mimeType {
                        Some(ref s) if s.contains("video") => "video",
                        Some(ref s) if s.contains("audio") => "audio",
                        Some(ref s) if s.contains("application") => "text",
                        _ => "unknown",
                    },
                };

                for rep in adaptation.representations.iter() {
                    let representation_id = rep.id.clone().unwrap_or("No ID".to_owned());

                    let representation_description = describe_representation(rep, adaptation);

                    debug!(
                        "\n  Representation {}: {}",
                        representation_id, representation_description
                    );

                    let segments = parse_segment_template::parse_segment_template(
                        &rep.SegmentTemplate,
                        &adaptation.SegmentTemplate,
                        &p.SegmentTemplate,
                        parse_segment_template::Context {
                            period_duration_ms: p.duration.map(|d| d.as_millis() as u64),
                        },
                    );

                    representations.push(ExpandedRepresentation { segments });
                }

                let adaptation_set = ExpandedAdaptationSet {
                    representations,
                    content_type: content_type.to_owned(),
                };

                adaptation_sets.push(adaptation_set);
            }

            let period = ExpandedPeriod {
                mpd_start_ms: period_start_ms,
                mpd_end_ms: period_end_ms,
                period_duration_ms,
                adaptation_sets,
                id: period_id,
            };

            previous_period_end_ms = period_end_ms;

            _periods.push(period);
        }

        ExpandedMpd { periods: _periods }
    }
}
