mod draw_queue;

use crate::util::error::DrawError;

use crate::debug;

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};

use draw_queue::DrawQueue;
use image::{GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;

use super::{Expanded, ExpandedMpd, ExpandedPeriod, ExpandedSegments};

type PixelSpacing = u32;

const IMAGE_PADDING: PixelSpacing = 60;

const SCALE: PixelSpacing = 40;

const PERIOD_TITLE_Y_SPACING: PixelSpacing = 30;
const PERIOD_TITLE_X_SPACING: PixelSpacing = 10;

const FONT_SIZE: f32 = 20.0;

const ADAPTATION_SET_PADDING: PixelSpacing = 0;
const ADAPTATION_SET_SPACING: PixelSpacing = SCALE / 2;

const REPRESENTATION_WIDTH: PixelSpacing = SCALE;
const REPRESENTATION_PADDING: PixelSpacing = 5;

const GAP_SIZE: i32 = 50;

enum Color {
    AudioSegmentOdd,
    AudioSegmentEvent,
    AudioAdaptationBorder,
    VideoSegmentOdd,
    VideoSegmentEven,
    VideoAdaptationBorder,
    Blue,
}

impl Color {
    pub fn to_rgba(self) -> (u8, u8, u8, u8) {
        match self {
            Color::AudioSegmentOdd => (144, 190, 109, 255),
            Color::AudioSegmentEvent => (169, 204, 142, 255),
            Color::AudioAdaptationBorder => (0, 255, 0, 255),
            Color::VideoSegmentOdd => (39, 125, 161, 255),
            Color::VideoSegmentEven => (47, 151, 196, 255),
            Color::VideoAdaptationBorder => (255, 0, 0, 255),
            Color::Blue => (0, 0, 255, 255),
        }
    }
}

struct DrawnPeriod<'a> {
    y_offset: i32,
    buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
    title_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
    period: &'a ExpandedPeriod,
}

impl ExpandedMpd {
    pub fn to_png(&mut self, debug: bool) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        let duration_ms = self.end_ms() - self.start_ms();

        if duration_ms > 600_000 {
            eprintln!("Manifest is > 10mins long. Will not parse");

            return None;
        }

        debug!(
            "Manifest is {}ms long ({} - {})",
            duration_ms,
            self.start_ms(),
            self.end_ms()
        );

        let canvas_height =
            ms_to_pixels(duration_ms, SCALE) + 2 * IMAGE_PADDING + PERIOD_TITLE_Y_SPACING;

        let mut drawn_periods: Vec<DrawnPeriod> = vec![];

        for period in self.periods.iter() {
            let period_width = get_period_width(period);
            let period_height = get_period_height(period);

            let mut period_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(
                period_width,
                period_height + 20,
                Rgba([255, 255, 255, 255]),
            );

            let mut y_offset: i32 = 0;

            if period.start_ms() > period.period_start_ms {
                y_offset = GAP_SIZE;

                draw_filled_rect_mut(
                    &mut period_buffer,
                    Rect::at(0, 0).of_size(period_width, GAP_SIZE as u32),
                    Rgba([230, 230, 230, 255]),
                );

                let text = &format!(
                    "{} gap",
                    &format_duration(period.start_ms() - period.period_start_ms)
                );

                let gap_font_size = 15f32;

                let (text_width, text_height) = text_dimensions(&font, text, gap_font_size);

                let x = (period_width - text_width) / 2;
                let y = (GAP_SIZE as u32 - text_height) / 2;

                draw_text_mut(
                    &mut period_buffer,
                    Rgba([0, 0, 0, 255]),
                    x as i32,
                    y as i32,
                    gap_font_size,
                    &font,
                    text,
                );
            }

            let y_offset: i32 = y_offset;

            // Track an x offset for all elements in the period
            let mut x_offset = 0;

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

            for (_index, adaptation) in period.adaptation_sets.iter().enumerate() {
                // Padding for the adaptation set
                x_offset += ADAPTATION_SET_PADDING;

                // Create a new draw queue
                let mut draw_queue = DrawQueue::new();

                // Draw all representations
                for representation in adaptation.representations.iter() {
                    let x = x_offset;

                    let width = REPRESENTATION_WIDTH;

                    // Slide the offset with padding
                    x_offset += width + REPRESENTATION_PADDING;

                    // let mut start_y = y_offset as i32;

                    match &representation.segments {
                        ExpandedSegments::SegmentTemplate { segment_timeline } => {
                            let x = x as i32 + 1;
                            let width = width - 2;

                            let mut i = 0;

                            let mut initial_y = y_offset as i32;

                            for segment in &segment_timeline.segments {
                                debug!(
                                    "Draw segment {} {} {} {} x {}ms {} @ {}",
                                    i,
                                    x,
                                    initial_y,
                                    period_height,
                                    segment.segment_duration_ms,
                                    segment.segment_count,
                                    segment.start_ms
                                );

                                let segment_end_y = initial_y
                                    + ms_to_pixels(segment.duration_ms, SCALE) as i32
                                    - 1i32;

                                // Draw each individual segment
                                for j in 0..segment.segment_count {
                                    let y0 = initial_y
                                        + ms_to_pixels(j * segment.segment_duration_ms, SCALE)
                                            as i32;

                                    let y1 = initial_y
                                        + ms_to_pixels((j + 1) * segment.segment_duration_ms, SCALE)
                                            as i32;

                                    let height = y1 - y0;

                                    if height < 1 {
                                        debug!("Less than 1px segment");
                                    } else {
                                        let (r, g, b, a) = match adaptation.content_type.as_str() {
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

                                        draw_filled_rect_mut(
                                            &mut period_buffer,
                                            Rect::at(x, y0).of_size(width, height as u32),
                                            Rgba([r, g, b, a]),
                                        );

                                        // start_y = y1;
                                        i += 1;
                                    }
                                }

                                initial_y = segment_end_y + 1;

                                draw_line_segment_mut(
                                    &mut period_buffer,
                                    (x as f32 + (width as f32 / 4.0), segment_end_y as f32),
                                    (
                                        x as f32 + width as f32 - 1f32 - (width as f32 / 4.0),
                                        segment_end_y as f32,
                                    ),
                                    Rgba([0, 0, 0, 255]),
                                )
                            }
                        }
                        _ => debug!("None segment timeline encountered"),
                    }

                    // Border the AdaptationSet

                    if debug {
                        let color = match adaptation.content_type.as_str() {
                            "video" => Color::VideoAdaptationBorder,
                            "audio" => Color::AudioAdaptationBorder,
                            _ => Color::Blue,
                        };

                        draw_queue.schedule(
                            x as i32,
                            y_offset,
                            width,
                            period_height - y_offset as u32,
                            color.to_rgba(),
                            false,
                        );
                    }
                }

                // offset -= REPRESENTATION_PADDING;

                x_offset += ADAPTATION_SET_PADDING;

                // (width + 1) * index as u32;
                x_offset += ADAPTATION_SET_SPACING;

                // Execute all scheduled drawing operations
                draw_queue.execute(&mut period_buffer);
            }

            draw_hollow_rect_mut(
                &mut period_buffer,
                Rect::at(0i32, 0i32).of_size(
                    period_width,
                    ms_to_pixels(period.end_ms() - period.start_ms(), SCALE) + y_offset as u32,
                ),
                Rgba([0, 0, 0, 255]),
            );

            // if period.offset_ms() > 0 {
            //     draw_translucent_rect(
            //         &mut period_buffer,
            //         Rect::at(0 as i32, 0)
            //             .of_size(period_width, ms_to_pixels(period.offset_ms(), SCALE) + 1),
            //         Rgba([255, 255, 255, 150]),
            //     );
            // }

            // draw_translucent_rect(
            //     &mut period_buffer,
            //     Rect::at(
            //         0i32,
            //         ms_to_pixels(period.end_ms() - period.start_ms(), SCALE) as i32
            //             + ms_to_pixels(period.offset_ms(), SCALE) as i32,
            //     )
            //     .of_size(
            //         period_width,
            //         20 + ms_to_pixels(period.offset_ms(), SCALE) + 1,
            //     ),
            //     Rgba([255, 255, 255, 150]),
            // );

            drawn_periods.push(DrawnPeriod {
                buffer: period_buffer,
                title_buffer,
                period,
                y_offset,
            });

            debug!("Drawing period");
        }

        let combined_width: u32 = drawn_periods.iter().map(|p| p.buffer.width()).sum();

        let mut combined: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(
            combined_width + 2 * IMAGE_PADDING,
            canvas_height,
            Rgba([255, 255, 255, 255]),
        );

        let mut x_position = IMAGE_PADDING;
        let mut i = 0;

        let start_timestamp = self.periods.first().expect("No periods").start_ms();

        for p in drawn_periods.iter() {
            let y_position = ms_to_pixels(p.period.start_ms() - start_timestamp, SCALE)
                + IMAGE_PADDING
                - p.y_offset as u32;

            debug!(
                "Copying {} {} to {} {} max {} {}",
                p.buffer.width(),
                p.buffer.height(),
                x_position,
                y_position,
                combined.width(),
                combined.height()
            );

            combined
                .copy_from(
                    &p.title_buffer,
                    if i == 0 {
                        x_position
                    } else {
                        x_position + PERIOD_TITLE_X_SPACING
                    },
                    y_position,
                )
                .expect("Unable to copy drawing");

            combined
                .copy_from(&p.buffer, x_position, y_position + PERIOD_TITLE_Y_SPACING)
                .expect("Unable to copy drawing");

            x_position += p.buffer.width();
            i += 1;
        }

        debug!("Done");

        Some(combined)
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

fn get_period_height(period: &ExpandedPeriod) -> u32 {
    debug!(
        "Calc period height {} {:?} {} - {}",
        period.period_start_ms,
        period.period_duration_ms,
        period.end_ms(),
        period.start_ms()
    );
    let duration_ms = period.end_ms() - period.start_ms();

    let height = ms_to_pixels(duration_ms, SCALE) as u32;

    if period.start_ms() > period.period_start_ms {
        height + GAP_SIZE as u32
    } else {
        height
    }
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
        result.push_str(&format!("{}.{}ms ", seconds, remaining_ms));
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

fn get_period_width(period: &ExpandedPeriod) -> u32 {
    let mut width = 0u32;

    for adaptation_set in &period.adaptation_sets {
        width += 2 * ADAPTATION_SET_PADDING;

        for _representation in &adaptation_set.representations {
            width += REPRESENTATION_WIDTH + REPRESENTATION_PADDING;
        }

        width += ADAPTATION_SET_SPACING;
    }

    // Remove the trailing spacer
    width -= ADAPTATION_SET_SPACING;
    width -= REPRESENTATION_PADDING;

    return width;
}
