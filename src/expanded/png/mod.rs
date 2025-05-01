mod draw_queue;

use std::time::Instant;

use crate::util::{draw::text_dimensions, error};

use crate::Config;

use draw_queue::{DrawQueue, DrawTask, FONT};
use image::{ImageBuffer, Rgba};
use log::{debug, info, warn};

use super::{
    Expanded, ExpandedAdaptationSet, ExpandedEvent, ExpandedMpd, ExpandedPeriod,
    ExpandedRepresentation, ExpandedSegments,
};

enum Color {
    AudioSegmentOdd,
    AudioSegmentEvent,
    VideoSegmentOdd,
    VideoSegmentEven,
    TextSegmentOdd,
    TextSegmentEven,
}

impl Color {
    pub fn to_rgba(self) -> (u8, u8, u8, u8) {
        match self {
            Color::AudioSegmentOdd => (144, 190, 109, 255),
            Color::AudioSegmentEvent => (169, 204, 142, 255),
            Color::VideoSegmentOdd => (39, 125, 161, 255),
            Color::VideoSegmentEven => (47, 151, 196, 255),
            Color::TextSegmentOdd => (255, 179, 102, 255),
            Color::TextSegmentEven => (255, 204, 153, 255),
        }
    }
}

struct DrawnEvent {
    start_ms: u64,
    end_ms: u64,
    duration_ms: u64,
    id: String,
    uri: String,
}

struct DrawnPeriod {
    id: String,
    draw_queue: DrawQueue,
    annotation_queue: DrawQueue,
}

struct DrawnRepresentation {
    draw_queue: DrawQueue,
}

struct DrawnAdaptationSet {
    draw_queue: DrawQueue,
}

impl ExpandedMpd {
    fn prepare(&self, config: &Config) -> DrawQueue {
        let mut from_ms = config
            .from_ms
            .unwrap_or(self.start_timestamp_ms())
            .max(self.start_timestamp_ms());
        let to_ms = config
            .to_ms
            .unwrap_or(self.end_timestamp_ms())
            .min(self.end_timestamp_ms());

        if from_ms > to_ms {
            panic!("Range {}ms to {}ms is invalid. {0} > {1}", from_ms, to_ms);
        }

        if !config.slice && to_ms - from_ms > config.max_duration_ms {
            warn!(
                "Calculated range {} to {} ({}) is greater than max_duration_ms ({}). See --max-duration-ms and --slice",
                from_ms,
                to_ms,
                format_duration(to_ms - from_ms),
                format_duration(config.max_duration_ms)
            );

            from_ms = to_ms - config.max_duration_ms;
        }

        let from_ms = from_ms - from_ms % 1000;
        let to_ms = to_ms + (1000 - to_ms % 1000);

        let revised_config = Config {
            to_ms: Some(to_ms),
            from_ms: Some(from_ms),
            ..*config
        };

        let duration_ms = to_ms - from_ms;

        info!(
            "Manifest is {} long ({}ms to {}ms) Drawing {} - {} ({})",
            format_duration(self.end_ms() - self.start_ms()),
            self.start_ms(),
            self.end_ms(),
            format_duration(from_ms),
            format_duration(to_ms),
            format_duration(to_ms - from_ms)
        );

        let mut x_position = revised_config.image_padding_x;

        let mut draw_queue = DrawQueue::new();

        for period in self.periods.iter() {
            if period.mpd_start_ms + period.start_ms() > to_ms {
                continue;
            }

            if period.mpd_start_ms + period.end_ms() < from_ms {
                continue;
            }

            let drawn_start_ms = if period.mpd_start_ms + period.start_ms() < from_ms {
                from_ms - (period.mpd_start_ms + period.start_ms())
            } else {
                0
            };

            let trim_start_px = ms_to_pixels(drawn_start_ms, revised_config.scale);

            let trim_end_px: u32 = if period.mpd_start_ms + period.end_ms() > to_ms {
                ms_to_pixels(
                    period.mpd_start_ms + period.end_ms() - to_ms,
                    revised_config.scale,
                )
            } else {
                0
            };

            let drawn_period = draw_period(period, &revised_config);

            let trimmed_queue = drawn_period.draw_queue.trim(
                (0, trim_start_px as i32),
                (
                    drawn_period.draw_queue.width() as i32,
                    drawn_period.draw_queue.height() as i32 - trim_end_px as i32,
                ),
            );

            let trimmed_annotation_queue = drawn_period.annotation_queue.trim(
                (0, trim_start_px as i32),
                (
                    drawn_period.annotation_queue.width() as i32,
                    drawn_period.annotation_queue.height() as i32 - trim_end_px as i32,
                ),
            );

            let relative_start_ms = if trim_start_px > 0 {
                0
            } else {
                (period.mpd_start_ms + period.start_ms()) - from_ms
            };

            let y_position = revised_config.image_padding_y
                + ms_to_pixels(relative_start_ms, revised_config.scale);

            let draw_queue_width = trimmed_queue.width();
            let annotation_queue_width = trimmed_annotation_queue.width();

            draw_queue.queue(DrawTask::Copy {
                draw_queue: trimmed_queue,
                x: x_position,
                y: y_position,
            });

            draw_queue.queue(DrawTask::Copy {
                draw_queue: trimmed_annotation_queue,
                x: x_position + draw_queue_width,
                y: y_position,
            });

            let (_, title_height) =
                text_dimensions(&*FONT, &drawn_period.id, revised_config.font_size);

            let title_y_position = ms_to_pixels(
                match relative_start_ms % 1000 {
                    0 => relative_start_ms + 100,
                    _ => relative_start_ms + 1000 - (relative_start_ms) % 1000 + 100,
                },
                revised_config.scale,
            ) + revised_config.image_padding_y;

            draw_queue.queue(DrawTask::Text {
                x: x_position as i32
                    + draw_queue_width as i32
                    + annotation_queue_width as i32
                    + revised_config.period_title_x_spacing as i32,
                y: title_y_position as i32,
                scale: revised_config.font_size,
                rgba: (0, 0, 0, 255),
                text: drawn_period.id,
            });

            if period.mpd_start_ms < from_ms {
                draw_queue.queue(DrawTask::Text {
                    x: x_position as i32
                        + draw_queue_width as i32
                        + annotation_queue_width as i32
                        + revised_config.period_title_x_spacing as i32,
                    y: title_y_position as i32 + title_height as i32 * 2,
                    scale: revised_config.font_size,
                    rgba: (0, 0, 0, 255),
                    text: format!("Starts at {}", format_duration(period.mpd_start_ms)),
                });

                draw_queue.queue(DrawTask::Text {
                    x: x_position as i32
                        + draw_queue_width as i32
                        + annotation_queue_width as i32
                        + revised_config.period_title_x_spacing as i32,
                    y: title_y_position as i32 + title_height as i32 * 4,
                    scale: revised_config.font_size,
                    rgba: (0, 0, 0, 255),
                    text: format!(
                        "Trimmed first {}",
                        format_duration(from_ms - period.mpd_start_ms)
                    ),
                });
            }

            x_position += draw_queue_width;
            // i += 1;
        }

        let draw_queue_width = draw_queue.width();
        let draw_queue_height = draw_queue.height();

        draw_queue.queue(DrawTask::HollowRect {
            x: 0,
            y: 0,
            width: draw_queue_width + revised_config.image_padding_x,
            height: draw_queue_height + revised_config.image_padding_y,
            rgba: (0, 0, 0, 255),
        });

        // Draw lines for each whole second in the manifest
        for i in (0..=duration_ms).step_by(1000) {
            let y_position = ms_to_pixels(i, revised_config.scale) as f32
                + revised_config.image_padding_y as f32;

            draw_queue.push(DrawTask::Line {
                start: (0f32, y_position),
                end: (
                    // IMAGE_PADDING as f32,
                    draw_queue.width() as f32,
                    // draw_queue.height() as f32 - IMAGE_PADDING as f32 - PERIOD_TITLE_Y_SPACING as f32,
                    y_position,
                ),
                rgba: (200, 200, 200, 255),
            });

            let label_text = format!("{}", (i + from_ms) / 1000);

            let (_, label_height) =
                text_dimensions(&*FONT, &label_text, revised_config.font_size / 1.5);

            draw_queue.push(DrawTask::Text {
                x: 10i32,
                y: y_position as i32 - label_height as i32 - 2,
                scale: revised_config.font_size / 1.5,
                rgba: (200, 200, 200, 255),
                text: label_text.clone(),
            });
        }

        let warning = String::from("Experimental. Verify output");

        let (warning_width, warning_height) =
            text_dimensions(&*FONT, &warning, revised_config.font_size);

        draw_queue.queue(DrawTask::Text {
            x: (draw_queue.width / 2 - warning_width / 2) as i32,
            y: (revised_config.image_padding_y / 2 - warning_height / 2) as i32,
            scale: revised_config.font_size,
            rgba: (255, 0, 0, 255),
            text: "Experimental. Verify output.".to_owned(),
        });

        draw_queue
    }

    pub fn to_plan(&self, config: &Config) -> String {
        let draw_queue = self.prepare(config);

        draw_queue.plan()
    }

    pub fn to_png(&self, config: &Config) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let start_time = Instant::now();

        let draw_queue = self.prepare(config);

        let background = ImageBuffer::from_pixel(
            draw_queue.width(),
            draw_queue.height(),
            Rgba([255, 255, 255, 255]),
        );

        // let combined = draw_queue.execute();
        let combined = draw_queue.execute_with_buffer(background);

        let elapsed = start_time.elapsed();

        info!("Time: {:?}", elapsed);

        Some(combined)
    }
}

fn draw_period(period: &ExpandedPeriod, config: &Config) -> DrawnPeriod {
    // Create a new draw queue
    let mut draw_queue = DrawQueue::new();

    let mut offset_x = 0u32;

    for adaptation_set in period.adaptation_sets.iter() {
        let drawn_adaptation_set =
            draw_adaptation_set(&adaptation_set, period.start_ms(), period.end_ms(), config);

        let width = drawn_adaptation_set.draw_queue.width();

        draw_queue.queue(DrawTask::Copy {
            draw_queue: drawn_adaptation_set.draw_queue,
            x: offset_x,
            y: 0,
        });

        offset_x += width + config.adaptation_set_padding;
    }

    let top_left = (0f32, 0f32);
    let bottom_left = (0f32, draw_queue.height() as f32 - 1.0);
    let top_right = (draw_queue.width() as f32 - 1.0, 0f32);
    let bottom_right = (
        draw_queue.width() as f32 - 1.0,
        draw_queue.height() as f32 - 1.0,
    );

    // Left
    draw_queue.queue(DrawTask::Line {
        start: top_left,
        end: bottom_left,
        rgba: (0, 0, 0, 255),
    });

    // Right
    draw_queue.queue(DrawTask::Line {
        start: top_right,
        end: bottom_right,
        rgba: (0, 0, 0, 255),
    });

    if let Some(from_ms) = config.from_ms {
        if from_ms <= period.mpd_start_ms {
            // Top
            draw_queue.queue(DrawTask::Line {
                start: top_left,
                end: top_right,
                rgba: (0, 0, 0, 255),
            });
        }
    }

    if let Some(to_ms) = config.to_ms {
        if to_ms >= period.mpd_end_ms {
            // Bottom
            draw_queue.queue(DrawTask::Line {
                start: bottom_left,
                end: bottom_right,
                rgba: (0, 0, 0, 255),
            });
        }
    }

    let annotation_queue = DrawQueue::new();

    // for event in period.events.iter() {
    //     // Ignore events which end before the segment timeline
    //     if event.end_ms < period.start_ms() {
    //         debug!(
    //             "Ignoring Event at {} - {} which ends before first segment at {}",
    //             event.start_ms,
    //             event.end_ms,
    //             period.start_ms()
    //         );
    //         continue;
    //     }

    //     let x_position = 10.0;
    //     let start_y = ms_to_pixels(event.start_ms - period.start_ms(), config.scale) as f32;

    //     annotation_queue.queue(DrawTask::Line {
    //         start: (x_position, start_y),
    //         end: (x_position + 20.0, start_y),
    //         rgba: (255, 0, 0, 255),
    //     });

    //     if event.duration_ms > 0 {
    //         let end_y = ms_to_pixels(event.end_ms - period.start_ms(), config.scale) as f32;

    //         annotation_queue.queue(DrawTask::Line {
    //             start: (x_position + 20.0, start_y),
    //             end: (x_position + 20.0, end_y),
    //             rgba: (255, 0, 0, 255),
    //         });

    //         annotation_queue.queue(DrawTask::Line {
    //             start: (x_position, end_y),
    //             end: (x_position + 20.0, end_y),
    //             rgba: (255, 0, 0, 255),
    //         });
    //     }
    // }

    DrawnPeriod {
        draw_queue: draw_queue,
        id: period.id.clone(),
        annotation_queue,
    }
}

fn draw_representation(
    representation: &ExpandedRepresentation,
    content_type: &str,
    period_start_ms: u64,
    period_end_ms: u64,
    config: &Config,
) -> DrawnRepresentation {
    let mut representation_queue = DrawQueue::new();

    match &representation.segments {
        ExpandedSegments::SegmentTemplate { segment_timeline } => {
            let width = config.representation_width;

            let mut i = 0;
            let mut initial_y =
                ms_to_pixels(representation.start_ms() - period_start_ms, config.scale) as i32;

            if representation.start_ms() - period_start_ms > 200 {
                debug!(
                    "Representation starts with a {} gap",
                    format_duration(representation.start_ms() - period_start_ms)
                );

                representation_queue.queue(DrawTask::FilledRect {
                    x: 0,
                    y: 0,
                    width: width,
                    height: initial_y as u32,
                    rgba: (200, 200, 200, 255),
                    hatch: Some((150, 150, 150, 255)),
                });
            }

            for segment in &segment_timeline.segments {
                let segment_end_y =
                    initial_y + ms_to_pixels(segment.duration_ms, config.scale) as i32 - 1i32;

                // Draw each individual segment
                for j in 0..segment.segment_count {
                    let y0 = initial_y
                        + ms_to_pixels(j * segment.segment_duration_ms, config.scale) as i32;

                    let y1 = initial_y
                        + ms_to_pixels((j + 1) * segment.segment_duration_ms, config.scale) as i32;

                    let height = y1 - y0;

                    if height < 1 {
                        warn!("Less than 1px segment");
                    } else {
                        let (r, g, b, a) = match content_type {
                            "audio" => match i % 2 {
                                0 => Color::AudioSegmentEvent.to_rgba(),
                                _ => Color::AudioSegmentOdd.to_rgba(),
                            },
                            "video" => match i % 2 {
                                0 => Color::VideoSegmentEven.to_rgba(),
                                _ => Color::VideoSegmentOdd.to_rgba(),
                            },
                            "text" => match i % 2 {
                                0 => Color::TextSegmentEven.to_rgba(),
                                _ => Color::TextSegmentOdd.to_rgba(),
                            },
                            _ => (255, 255, 0, 255),
                        };

                        representation_queue.queue(DrawTask::FilledRect {
                            x: 0,
                            y: y0,
                            width: width,
                            height: height as u32,
                            rgba: (r, g, b, a),
                            hatch: None,
                        });

                        // start_y = y1;
                        i += 1;
                    }
                }

                initial_y = segment_end_y + 1;

                representation_queue.queue(DrawTask::Line {
                    start: ((width as f32 / 4.0), segment_end_y as f32),
                    end: (
                        width as f32 - 1f32 - (width as f32 / 4.0),
                        segment_end_y as f32,
                    ),
                    rgba: (0, 0, 0, 255),
                });
            }

            if representation.end_ms() < period_end_ms
                && period_end_ms - representation.end_ms() > 200
            {
                debug!(
                    "Representation ends with a {} gap",
                    format_duration(period_end_ms - representation.end_ms())
                );

                representation_queue.queue(DrawTask::FilledRect {
                    x: 0,
                    y: initial_y,
                    width: width,
                    height: ms_to_pixels(period_end_ms - representation.end_ms(), config.scale),
                    rgba: (200, 200, 200, 255),
                    hatch: Some((150, 150, 150, 255)),
                });
            }
        }
        _ => warn!("None segment timeline encountered"),
    }

    DrawnRepresentation {
        draw_queue: representation_queue,
    }
}

fn draw_adaptation_set(
    adaptation_set: &ExpandedAdaptationSet,
    period_start_ms: u64,
    period_end_ms: u64,
    config: &Config,
) -> DrawnAdaptationSet {
    // let mut drawn_representations: Vec<DrawnRepresentation> = vec![];
    let mut draw_queue = DrawQueue::new();

    // Draw all representations
    for (index, representation) in adaptation_set.representations.iter().enumerate() {
        let drawn_representation = draw_representation(
            representation,
            &adaptation_set.content_type,
            period_start_ms,
            period_end_ms,
            config,
        );

        draw_queue.queue(DrawTask::Copy {
            draw_queue: drawn_representation.draw_queue,
            x: index as u32 * (config.representation_width + config.representation_padding),
            y: 0,
        })
    }

    DrawnAdaptationSet {
        draw_queue: draw_queue,
    }
}

fn ms_to_pixels(ms: u64, scale: u32) -> u32 {
    // Separate the duration into whole ms and fractional ms
    let _ms = ms % 1000;
    let s = (ms - _ms as u64) / 1000;

    let ms = _ms as u32;
    let s = s as u32;

    // Calculate pixels for whole ms
    let whole_seconds_pixels = s * scale;

    let pc = ms as f32 / 1000.0;

    let px = (pc * scale as f32).round() as u32;

    // Sum both parts to get the total pixel width
    whole_seconds_pixels + px
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return format!("0s");
    }

    let mut remaining_ms = duration_ms;
    let mut result = String::new();

    let years = remaining_ms / (1000 * 60 * 60 * 24 * 365);
    if years > 0 {
        result.push_str(&format!("{}yr ", years));
        remaining_ms %= 1000 * 60 * 60 * 24 * 365;
    }

    let months = remaining_ms / (1000 * 60 * 60 * 24 * 30);
    if months > 0 {
        result.push_str(&format!("{}mo ", months));
        remaining_ms %= 1000 * 60 * 60 * 24 * 30;
    }

    let days = remaining_ms / (1000 * 60 * 60 * 24);
    if days > 0 {
        result.push_str(&format!("{}day ", days));
        remaining_ms %= 1000 * 60 * 60 * 24;
    }

    let hours = remaining_ms / (1000 * 60 * 60);
    if hours > 0 {
        result.push_str(&format!("{}hr ", hours));
        remaining_ms %= 1000 * 60 * 60;
    }

    let minutes = remaining_ms / (1000 * 60);
    if minutes > 0 {
        result.push_str(&format!("{}min ", minutes));
        remaining_ms %= 1000 * 60;
    }

    let seconds = remaining_ms / 1000;
    remaining_ms %= 1000;

    if seconds > 0 {
        result.push_str(&format!("{}.{}s ", seconds, remaining_ms));
    } else if remaining_ms > 0 {
        result.push_str(&format!("{}ms ", remaining_ms));
    }

    result.trim().to_string()
}

// // Function to draw a translucent rectangle
// fn draw_translucent_rect(img: &mut RgbaImage, rect: Rect, color: Rgba<u8>) {
//     for y in rect.top()..rect.bottom() {
//         for x in rect.left()..rect.right() {
//             if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
//                 let px = img.get_pixel_mut(x as u32, y as u32);
//                 blend_pixel(px, color);
//             }
//         }
//     }
// }

// // Function to blend a pixel with a translucent color
// fn blend_pixel(pixel: &mut Rgba<u8>, overlay: Rgba<u8>) {
//     let alpha = overlay.0[3] as f32 / 255.0;
//     for i in 0..3 {
//         pixel.0[i] = (pixel.0[i] as f32 * (1.0 - alpha) + overlay.0[i] as f32 * alpha) as u8;
//     }
// }
