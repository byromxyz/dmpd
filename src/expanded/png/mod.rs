mod draw_queue;

use crate::util::parse::parse_ms_duration;
use crate::util::{draw::text_dimensions, error::DrawError};

use crate::{debug, Config};

use ab_glyph::FontRef;

use draw_queue::{DrawQueue, DrawTask};
use image::{ImageBuffer, Rgba};

use super::{
    Expanded, ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation,
    ExpandedSegments,
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

struct DrawnPeriod {
    id: String,
    draw_queue: DrawQueue,
}

struct DrawnRepresentation {
    draw_queue: DrawQueue,
}

struct DrawnAdaptationSet {
    draw_queue: DrawQueue,
}

impl ExpandedMpd {
    fn prepare(&self, config: &Config) -> DrawQueue {
        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

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
            println!(
                "Calculated range {} to {} ({}ms) is greater than max_duration_ms ({}ms). See --max-duration-ms and --slice",
                from_ms,
                to_ms,
                to_ms - from_ms,
                config.max_duration_ms
            );

            from_ms = to_ms - config.max_duration_ms;
        }

        let from_ms = from_ms - from_ms % 1000;
        let to_ms = to_ms + (1000 - to_ms % 1000);

        let duration_ms = to_ms - from_ms;

        println!(
            "Manifest is {} long ({}ms to {}ms). Drawing {}ms - {}ms ({}ms)",
            parse_ms_duration(self.end_ms() - self.start_ms()),
            self.start_ms(),
            self.end_ms(),
            from_ms,
            to_ms,
            to_ms - from_ms
        );

        let mut x_position = config.image_padding_x;

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

            let trim_start_px = ms_to_pixels(drawn_start_ms, config.scale);

            let trim_end_px: u32 = if period.mpd_start_ms + period.end_ms() > to_ms {
                ms_to_pixels(period.mpd_start_ms + period.end_ms() - to_ms, config.scale)
            } else {
                0
            };

            let drawn_period = draw_period(period, config);

            let trimmed_queue = drawn_period.draw_queue.trim(
                (0, trim_start_px as i32),
                (
                    drawn_period.draw_queue.width() as i32,
                    drawn_period.draw_queue.height() as i32 - trim_end_px as i32,
                ),
            );

            let relative_start_ms = if trim_start_px > 0 {
                0
            } else {
                (period.mpd_start_ms + period.start_ms()) - from_ms
            };

            let y_position = config.image_padding_y + ms_to_pixels(relative_start_ms, config.scale);

            let draw_queue_width = trimmed_queue.width();

            draw_queue.queue(DrawTask::Copy {
                draw_queue: trimmed_queue,
                x: x_position,
                y: y_position,
            });

            let (title_width, title_height) =
                text_dimensions(&font, &drawn_period.id, config.font_size);

            let title_y_position = ms_to_pixels(
                match relative_start_ms % 1000 {
                    0 => relative_start_ms + 100,
                    _ => relative_start_ms + 1000 - (relative_start_ms) % 1000 + 100,
                },
                config.scale,
            ) + config.image_padding_y;

            draw_queue.queue(DrawTask::Text {
                x: x_position as i32
                    + draw_queue_width as i32
                    + config.period_title_x_spacing as i32,
                y: title_y_position as i32,
                scale: config.font_size,
                rgba: (0, 0, 0, 255),
                text: drawn_period.id,
                width: title_width,
                height: title_height,
            });

            x_position += draw_queue_width;
            // i += 1;
        }

        let draw_queue_width = draw_queue.width();
        let draw_queue_height = draw_queue.height();

        draw_queue.queue(DrawTask::HollowRect {
            x: 0,
            y: 0,
            width: draw_queue_width + config.image_padding_x,
            height: draw_queue_height + config.image_padding_y,
            rgba: (0, 0, 0, 255),
        });

        // Draw lines for each whole second in the manifest
        for i in (0..=duration_ms).step_by(1000) {
            let y_position = ms_to_pixels(i, config.scale) as f32 + config.image_padding_y as f32;

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

            let (title_width, title_height) =
                text_dimensions(&font, &label_text, config.font_size / 1.5);

            draw_queue.push(DrawTask::Text {
                x: 10i32,
                y: y_position as i32 - title_height as i32 - 2,
                scale: config.font_size / 1.5,
                rgba: (200, 200, 200, 255),
                text: label_text.clone(),
                width: title_width,
                height: title_height,
            });
        }

        draw_queue
    }

    pub fn to_plan(&self, config: &Config) -> String {
        let draw_queue = self.prepare(config);

        draw_queue.plan()
    }

    pub fn to_png(&self, config: &Config) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let draw_queue = self.prepare(config);

        let background = ImageBuffer::from_pixel(
            draw_queue.width(),
            draw_queue.height(),
            Rgba([255, 255, 255, 255]),
        );

        // let combined = draw_queue.execute();
        let combined = draw_queue.execute_with_buffer(background);

        debug!("Done");

        Some(combined)
    }
}

fn draw_period(period: &ExpandedPeriod, config: &Config) -> DrawnPeriod {
    let start_ms = period.start_ms();

    // Create a new draw queue
    let mut draw_queue = DrawQueue::new();

    let mut offset_x = 0u32;

    for adaptation_set in period.adaptation_sets.iter() {
        let drawn_adaptation_set = draw_adaptation_set(&adaptation_set, start_ms, config);

        let width = drawn_adaptation_set.draw_queue.width();

        draw_queue.queue(DrawTask::Copy {
            draw_queue: drawn_adaptation_set.draw_queue,
            x: offset_x,
            y: 0,
        });

        offset_x += width + config.adaptation_set_padding;
    }

    draw_queue.queue(DrawTask::HollowRect {
        x: 0,
        y: 0,
        width: draw_queue.width(),
        height: draw_queue.height(),
        rgba: (0, 0, 0, 255),
    });

    DrawnPeriod {
        draw_queue: draw_queue,
        id: period.id.clone(),
    }
}

fn draw_representation(
    representation: &ExpandedRepresentation,
    content_type: &str,
    start_ms: u64,
    config: &Config,
) -> DrawnRepresentation {
    let mut representation_queue = DrawQueue::new();

    match &representation.segments {
        ExpandedSegments::SegmentTemplate { segment_timeline } => {
            let width = config.representation_width;

            let mut i = 0;
            let mut initial_y =
                ms_to_pixels(representation.start_ms() - start_ms, config.scale) as i32;

            for segment in &segment_timeline.segments {
                debug!(
                    "Draw segment {} {} x {}ms {} @ {}",
                    i,
                    initial_y,
                    segment.segment_duration_ms,
                    segment.segment_count,
                    segment.start_ms
                );

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
                        debug!("Less than 1px segment");
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
        }
        _ => debug!("None segment timeline encountered"),
    }

    DrawnRepresentation {
        draw_queue: representation_queue,
    }
}

fn draw_adaptation_set(
    adaptation_set: &ExpandedAdaptationSet,
    start_ms: u64,
    config: &Config,
) -> DrawnAdaptationSet {
    // let mut drawn_representations: Vec<DrawnRepresentation> = vec![];
    let mut draw_queue = DrawQueue::new();

    // Draw all representations
    for (index, representation) in adaptation_set.representations.iter().enumerate() {
        let drawn_representation = draw_representation(
            representation,
            &adaptation_set.content_type,
            start_ms,
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
