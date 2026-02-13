use core::time;

use chrono::{DateTime, Duration, Utc};
use dash_mpd::{Event, Period, MPD};
use log::debug;

use crate::{
    expanded::{Expanded, ExpandedEvent, MpdType},
    util::{parse::describe_representation, parse_segment_template},
};

use super::{ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation};

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
    mpd_type: &MpdType,
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

    match mpd_type {
        MpdType::Dynamic {
            availability_start_time,
            publish_time,
            minimum_update_period,
            suggested_presentation_delay: _,
        } => {
            // Calculate the latest possible "now" based on the publish time and update period
            let now = *publish_time
                + Duration::from_std(*minimum_update_period)
                    .expect("Out of range whilst parsing minimum_update_period ??");

            // Calculate a maximum duration using availability_start_time as the zero-point
            let timeline_duration = (now - *availability_start_time).num_milliseconds() as u64;

            return timeline_duration - start_ms;
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

impl ExpandedMpd {
    pub fn new(mpd: MPD) -> Self {
        let mpd_type = match &mpd.mpdtype.as_deref() {
            Some("dynamic") => MpdType::Dynamic {
                availability_start_time: mpd.availabilityStartTime.unwrap(),
                publish_time: mpd.publishTime.unwrap(),
                minimum_update_period: mpd.minimumUpdatePeriod.unwrap(),
                suggested_presentation_delay: mpd.suggestedPresentationDelay,
            },
            Some("static") => MpdType::Static {
                media_presentation_duration: mpd.mediaPresentationDuration,
            },
            _ => MpdType::Static {
                media_presentation_duration: mpd.mediaPresentationDuration,
            },
        };

        let mut _periods: Vec<ExpandedPeriod> = vec![];

        let mut previous_period_end_ms = 0u64;

        for (i, p) in mpd.periods.iter().enumerate() {
            let next = mpd.periods.get(i + 1);

            let period_start_ms = get_period_start_ms(p, previous_period_end_ms);

            let period_duration_ms = get_period_duration_ms(p, next, period_start_ms, &mpd_type);
            let period_end_ms = period_start_ms + period_duration_ms;

            let period_id = p.id.clone().unwrap_or("No ID".to_owned());

            debug!("Expanding Period: {}", period_id);

            debug!("Contains {} AdaptationSets", p.adaptations.len());

            let mut adaptation_sets: Vec<ExpandedAdaptationSet> = vec![];

            for adaptation in p.adaptations.iter() {
                let adaptation_set_id = adaptation.id.clone().unwrap_or("No ID".to_owned());

                debug!(
                    "AdaptationSet {} ({}) has {} Representations",
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
                        " - Representation {}: {}",
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

            let events: Vec<ExpandedEvent> = p
                .event_streams
                .iter()
                .map(|event_stream| {
                    let timescale = event_stream.timescale.unwrap_or(1);
                    let presentation_time_offset = event_stream.presentationTimeOffset.unwrap_or(0);

                    let events: Vec<ExpandedEvent> = event_stream
                        .event
                        .iter()
                        .map(|event| {
                            let start_t =
                                event.presentationTime.unwrap_or(0) - presentation_time_offset;

                            let start_ms =
                                ((start_t as u128 * 1_000u128) / timescale as u128) as u64;

                            let duration_t = event.duration.unwrap_or(0);

                            let duration_ms =
                                ((duration_t as u128 * 1_000u128) / timescale as u128) as u64;

                            let end_ms = start_ms + duration_ms;

                            let scheme_id_uri = event_stream
                                .schemeIdUri
                                .clone()
                                .unwrap_or(format!("No URI"));

                            let id = event.id.clone().unwrap_or(format!("No ID"));

                            ExpandedEvent {
                                start_ms,
                                end_ms,
                                duration_ms,
                                id,
                                scheme_id_uri,
                            }
                        })
                        .collect();

                    events
                })
                .flatten()
                .collect();

            let timeline_duration_ms = adaptation_sets.last().unwrap().end_ms();

            let period = ExpandedPeriod {
                mpd_start_ms: period_start_ms,
                mpd_end_ms: period_end_ms,
                period_duration_ms: period_duration_ms.max(timeline_duration_ms),
                adaptation_sets,
                id: period_id,
                events,
            };

            previous_period_end_ms = period_end_ms;

            _periods.push(period);
        }

        ExpandedMpd {
            periods: _periods,
            mpd_type,
        }
    }
}
