use dash_mpd::MPD;

use crate::{
    debug,
    util::{
        error::ParseError,
        parse::{
            describe_representation, parse_period_duration_ms, parse_period_start_ms,
            parse_segment_template,
        },
    },
};

use super::{Expanded, ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation};

impl ExpandedMpd {
    pub fn new(mpd: MPD) -> Self {
        let mut _periods: Vec<ExpandedPeriod> = vec![];

        let mut previous_period_end_ms = 0u64;

        for p in mpd.periods {
            let period_id = p.id.clone().unwrap_or("No ID".to_owned());

            debug!("\nPeriod: {}", period_id);

            let period_start_ms = parse_period_start_ms(&p, previous_period_end_ms);
            let period_duration_ms: Option<u64> = parse_period_duration_ms(&p);

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
                    _ => panic!(
                        "{}",
                        &ParseError::AdaptationSetWithoutContentType.describe()
                    ),
                };

                for rep in adaptation.representations.iter() {
                    let representation_id = rep.id.clone().unwrap_or("No ID".to_owned());

                    let representation_description = describe_representation(rep, adaptation);

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
                    content_type: content_type.to_owned(),
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
