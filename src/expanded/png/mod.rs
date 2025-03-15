mod draw_queue;

use crate::util::error::DrawError;

use crate::debug;

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};

use draw_queue::{DrawQueue, DrawTask};
use image::{ImageBuffer, Rgba};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};

use super::{
    Expanded, ExpandedAdaptationSet, ExpandedMpd, ExpandedPeriod, ExpandedRepresentation,
    ExpandedSegments,
};

type PixelSpacing = u32;

const IMAGE_PADDING_X: PixelSpacing = 120;
const IMAGE_PADDING_Y: PixelSpacing = 60;

const SCALE: PixelSpacing = 40;

const PERIOD_TITLE_Y_SPACING: PixelSpacing = 30;
const PERIOD_TITLE_X_SPACING: PixelSpacing = 10;

const FONT_SIZE: f32 = 20.0;

const ADAPTATION_SET_PADDING: PixelSpacing = 20;

const REPRESENTATION_WIDTH: PixelSpacing = SCALE;
const REPRESENTATION_PADDING: PixelSpacing = 5;

const GAP_SIZE: i32 = 50;

enum Color {
    AudioSegmentOdd,
    AudioSegmentEvent,
    VideoSegmentOdd,
    VideoSegmentEven,
}

impl Color {
    pub fn to_rgba(self) -> (u8, u8, u8, u8) {
        match self {
            Color::AudioSegmentOdd => (144, 190, 109, 255),
            Color::AudioSegmentEvent => (169, 204, 142, 255),
            Color::VideoSegmentOdd => (39, 125, 161, 255),
            Color::VideoSegmentEven => (47, 151, 196, 255),
        }
    }
}

struct DrawnPeriod {
    title_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
    period: ExpandedPeriod,
    draw_queue: DrawQueue,
}

struct DrawnRepresentation {
    draw_queue: DrawQueue,
    representation: ExpandedRepresentation,
}

struct DrawnAdaptationSet {
    draw_queue: DrawQueue,
    adaptation_set: ExpandedAdaptationSet,
}

impl ExpandedMpd {
    fn prepare(&self) -> DrawQueue {
        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        let start_timestamp = self.start_timestamp_ms();
        let end_timestamp = self.end_timestamp_ms();

        let mut drawn_periods: Vec<DrawnPeriod> = vec![];

        for period in self.periods.iter() {
            debug!("Drawing period");
            let drawn_period = draw_period(period);

            drawn_periods.push(drawn_period);
        }

        let mut x_position = IMAGE_PADDING_X;
        let mut i = 0;

        let mut draw_queue = DrawQueue::new();

        for p in drawn_periods.iter() {
            let y_position = ms_to_pixels(
                p.period.start_ms + p.period.start_ms() - start_timestamp,
                SCALE,
            ) + IMAGE_PADDING_Y;

            draw_queue.queue(DrawTask::Copy {
                draw_queue: p.draw_queue.clone(),
                x: x_position,
                y: y_position + PERIOD_TITLE_Y_SPACING,
            });

            draw_queue.queue(DrawTask::Text {
                x: x_position as i32,
                y: y_position as i32 as i32,
                scale: FONT_SIZE,
                rgba: (0, 0, 0, 255),
                text: p.period.id.clone(),
            });

            x_position += p.draw_queue.width();
            i += 1;
        }

        draw_queue.queue(DrawTask::HollowRect {
            x: 0,
            y: 0,
            width: draw_queue.width() + IMAGE_PADDING_X,
            height: draw_queue.height() + IMAGE_PADDING_Y + PERIOD_TITLE_Y_SPACING,
            rgba: (0, 0, 0, 255),
        });

        draw_queue
    }

    pub fn to_plan(&mut self) -> String {
        let draw_queue = self.prepare();

        draw_queue.plan()
    }

    pub fn to_png(&mut self, _debug: bool) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let start_timestamp = self.start_timestamp_ms();
        let end_timestamp = self.end_timestamp_ms();

        let duration_ms = end_timestamp - start_timestamp;

        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        // if duration_ms > 600_000 {
        //     eprintln!("Manifest is > 10mins long. Will not parse");

        //     return None;
        // }

        println!(
            "Manifest is {}ms long ({} - {})",
            duration_ms,
            self.end_ms(),
            self.start_ms(),
        );

        let draw_queue = self.prepare();

        let mut background = ImageBuffer::from_pixel(
            draw_queue.width(),
            draw_queue.height(),
            Rgba([255, 255, 255, 255]),
        );

        let mut next_second = 1000 - start_timestamp % 1000;

        if next_second == 1000 {
            next_second = 0;
        }

        let mut last_second = end_timestamp - (end_timestamp % 1000) - start_timestamp;

        if end_timestamp % 1000 == 0 {
            last_second = end_timestamp - start_timestamp;
        }

        // Draw lines for each whole second in the manifest
        for i in (next_second..=last_second).step_by(1000) {
            let y_position = ms_to_pixels(i, SCALE) as f32
                + IMAGE_PADDING_Y as f32
                + PERIOD_TITLE_Y_SPACING as f32;

            draw_line_segment_mut(
                &mut background,
                (0f32, y_position),
                (
                    // IMAGE_PADDING as f32,
                    draw_queue.width() as f32,
                    // draw_queue.height() as f32 - IMAGE_PADDING as f32 - PERIOD_TITLE_Y_SPACING as f32,
                    y_position,
                ),
                Rgba([200, 200, 200, 255]),
            );

            let label_text = format!("{}", (i + start_timestamp) / 1000);

            let (title_width, title_height) = text_dimensions(&font, &label_text, FONT_SIZE / 1.5);

            draw_text_mut(
                &mut background,
                Rgba([200, 200, 200, 255]),
                10i32,
                y_position as i32 - title_height as i32 - 2,
                FONT_SIZE / 1.5,
                &font,
                &label_text,
            );
        }

        let combined = draw_queue.execute_with_buffer(background);

        debug!("Done");

        Some(combined)
    }
}

fn draw_period(period: &ExpandedPeriod) -> DrawnPeriod {
    let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
        .expect(&DrawError::CannotCreateFont.describe());

    let height = ms_to_pixels(period.end_ms() - period.start_ms(), SCALE);
    let start_ms = period.start_ms();

    let mut drawn_adaptation_sets: Vec<DrawnAdaptationSet> = vec![];

    for adaptation_set in period.adaptation_sets.iter() {
        let adaptation_set_buffer = draw_adaptation_set(&adaptation_set, height, start_ms);

        drawn_adaptation_sets.push(adaptation_set_buffer);
    }

    let period_width: u32 = drawn_adaptation_sets
        .iter()
        .map(|drawn_adaptation_set| {
            drawn_adaptation_set.draw_queue.width() + ADAPTATION_SET_PADDING
        })
        .sum();

    let period_width = period_width - ADAPTATION_SET_PADDING;

    let period_height = drawn_adaptation_sets
        .iter()
        .map(|drawn_adaptation_set| drawn_adaptation_set.draw_queue.height())
        .max()
        .unwrap_or(0);

    // Create a new draw queue
    let mut draw_queue = DrawQueue::new();

    // if period.start_ms() > 0 && period_index == 0 {
    //     draw_queue.queue(DrawTask::FilledRect {
    //         x: 0,
    //         y: 0,
    //         width: period_width,
    //         height: GAP_SIZE as u32,
    //         rgba: (255, 100, 100, 255),
    //     });

    //     let text = &format!("{} gap", &format_duration(period.start_ms()));

    //     let gap_font_size = 15f32;

    //     let (text_width, text_height) = text_dimensions(&font, text, gap_font_size);

    //     let x = (period_width - text_width) / 2;
    //     let y = (GAP_SIZE as u32 - text_height) / 2;

    //     draw_queue.queue(DrawTask::Text {
    //         x: x as i32,
    //         y: y as i32,
    //         scale: gap_font_size,
    //         rgba: (0, 0, 0, 255),
    //         text: text.to_string(),
    //     });
    // }

    let mut offset_x = 0u32;

    for drawn_adaptation_set in drawn_adaptation_sets.iter() {
        debug!("Copying buffer {} {}", offset_x, 0,);

        draw_queue.queue(DrawTask::Copy {
            draw_queue: drawn_adaptation_set.draw_queue.clone(), // TODO - clone okay?
            x: offset_x,
            y: 0,
        });

        offset_x += drawn_adaptation_set.draw_queue.width() + ADAPTATION_SET_PADDING;
    }

    draw_queue.queue(DrawTask::HollowRect {
        x: 0,
        y: 0,
        width: period_width,
        height: period_height,
        rgba: (0, 0, 0, 255),
    });

    // let period_buffer = draw_queue.execute();

    let (title_width, title_height) = text_dimensions(&font, &period.id, FONT_SIZE);

    let mut title_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(title_width, title_height * 2, Rgba([255, 255, 255, 255]));

    draw_text_mut(
        &mut title_buffer,
        Rgba([0, 0, 0, 255]),
        0,
        title_height as i32 / 4,
        FONT_SIZE,
        &font,
        &period.id,
    );

    DrawnPeriod {
        title_buffer,
        period: period.clone(),
        draw_queue,
    }
}

fn draw_representation(
    representation: &ExpandedRepresentation,
    content_type: &str,
    start_ms: u64,
) -> DrawnRepresentation {
    let mut representation_queue = DrawQueue::new();

    match &representation.segments {
        ExpandedSegments::SegmentTemplate { segment_timeline } => {
            let width = REPRESENTATION_WIDTH;

            let mut i = 0;
            let mut initial_y = ms_to_pixels(representation.start_ms() - start_ms, SCALE) as i32;

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
                    initial_y + ms_to_pixels(segment.duration_ms, SCALE) as i32 - 1i32;

                // Draw each individual segment
                for j in 0..segment.segment_count {
                    let y0 =
                        initial_y + ms_to_pixels(j * segment.segment_duration_ms, SCALE) as i32;

                    let y1 = initial_y
                        + ms_to_pixels((j + 1) * segment.segment_duration_ms, SCALE) as i32;

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
        representation: representation.clone(),
    }
}

fn draw_adaptation_set(
    adaptation_set: &ExpandedAdaptationSet,
    height: u32,
    start_ms: u64,
) -> DrawnAdaptationSet {
    let mut drawn_representations: Vec<DrawnRepresentation> = vec![];

    // Draw all representations
    for representation in adaptation_set.representations.iter() {
        let drawn_representation =
            draw_representation(representation, &adaptation_set.content_type, start_ms);

        drawn_representations.push(drawn_representation);
    }

    let mut draw_queue = DrawQueue::new();

    for (index, drawn_representation) in drawn_representations.iter().enumerate() {
        debug!(
            "Copying buffer {} {} {}x{}",
            index as u32 * (REPRESENTATION_WIDTH + REPRESENTATION_PADDING),
            0,
            drawn_representation.draw_queue.width(),
            drawn_representation.draw_queue.height(),
        );

        draw_queue.queue(DrawTask::Copy {
            draw_queue: drawn_representation.draw_queue.clone(),
            x: index as u32 * (REPRESENTATION_WIDTH + REPRESENTATION_PADDING),
            y: 0,
        })
    }

    DrawnAdaptationSet {
        draw_queue,
        adaptation_set: adaptation_set.clone(),
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

/// Estimates the size of a given `text` string at `font_size` in `font``.
/// ImageProc does not expose the expected size of draw_text_mut so this function is copied from source
/// https://github.com/image-rs/imageproc/blob/master/src/drawing/text.rs#L10-L37

fn text_dimensions(font: &impl Font, text: &str, font_size: f32) -> (u32, u32) {
    let scale = PxScale::from(font_size);

    let (mut w, mut h) = (0f32, 0f32);

    let font = font.as_scaled(scale);
    let mut last: Option<GlyphId> = None;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = glyph_id.with_scale_and_position(scale, point(w, font.ascent()));
        w += font.h_advance(glyph_id);
        if let Some(g) = font.outline_glyph(glyph) {
            if let Some(last) = last {
                w += font.kern(glyph_id, last);
            }
            last = Some(glyph_id);
            let bb = g.px_bounds();
            h = h.max(bb.height());
        }
    }

    (w as u32, h as u32)
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
