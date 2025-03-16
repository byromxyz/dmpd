use ab_glyph::FontRef;
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};
use serde::Serialize;

use crate::util::error::DrawError;

/// Define a structure to hold a queue of draw operations
#[derive(Debug, Clone, Serialize)]
pub struct DrawQueue {
    queue: Vec<DrawTask>,
}

impl DrawQueue {
    pub fn width(&self) -> u32 {
        self.queue
            .iter()
            .map(|task| match task {
                DrawTask::Line { end, .. } => {
                    return end.0 as u32;
                }
                DrawTask::FilledRect { x, width, .. } => {
                    return (x + *width as i32) as u32;
                }
                DrawTask::HollowRect { x, width, .. } => {
                    return (*x + *width as i32) as u32;
                }
                DrawTask::Text { x, text, .. } => {
                    return (*x + text.len() as i32) as u32;
                }
                DrawTask::Copy {
                    draw_queue,
                    x,
                    y: _,
                } => {
                    return *x + draw_queue.width();
                }
            })
            .max()
            .unwrap_or(0)
    }

    pub fn height(&self) -> u32 {
        self.queue
            .iter()
            .map(|task| match task {
                DrawTask::Line { end, .. } => {
                    return end.1 as u32;
                }
                DrawTask::FilledRect { y, height, .. } => {
                    return (*y + *height as i32) as u32;
                }
                DrawTask::HollowRect { y, height, .. } => {
                    return (*y + *height as i32) as u32;
                }
                DrawTask::Text { y, scale, .. } => {
                    return (*y + *scale as i32) as u32;
                }
                DrawTask::Copy {
                    draw_queue,
                    x: _,
                    y,
                } => {
                    return *y + draw_queue.height();
                }
            })
            .max()
            .unwrap_or(0)
    }

    pub fn plan(&self) -> String {
        let json = serde_json::to_string_pretty(self).unwrap();

        json
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum DrawTask {
    FilledRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        rgba: (u8, u8, u8, u8),
    },
    HollowRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        rgba: (u8, u8, u8, u8),
    },
    Text {
        x: i32,
        y: i32,
        scale: f32,
        rgba: (u8, u8, u8, u8),
        text: String,
    },
    Line {
        start: (f32, f32),
        end: (f32, f32),
        rgba: (u8, u8, u8, u8),
    },
    Copy {
        draw_queue: DrawQueue,
        x: u32,
        y: u32,
    },
}

fn blend_pixel(base: &mut Rgba<u8>, overlay: Rgba<u8>) {
    let [r_o, g_o, b_o, a_o] = overlay.0;
    let alpha = a_o as f32 / 255.0;

    if a_o == 0 {
        return; // nothing to blend
    }

    let [r_b, g_b, b_b, a_b] = base.0;
    let alpha_b = a_b as f32 / 255.0;

    let out_alpha = alpha + alpha_b * (1.0 - alpha);
    let r = ((r_o as f32 * alpha + r_b as f32 * alpha_b * (1.0 - alpha)) / out_alpha).round() as u8;
    let g = ((g_o as f32 * alpha + g_b as f32 * alpha_b * (1.0 - alpha)) / out_alpha).round() as u8;
    let b = ((b_o as f32 * alpha + b_b as f32 * alpha_b * (1.0 - alpha)) / out_alpha).round() as u8;
    let a = (out_alpha * 255.0).round() as u8;

    *base = Rgba([r, g, b, a]);
}

/// An alternative to copy_from which respects alpha channels
fn blend_images(dest: &mut RgbaImage, src: &RgbaImage, offset_x: u32, offset_y: u32) {
    for (x, y, overlay_pixel) in src.enumerate_pixels() {
        let dx = x + offset_x;
        let dy = y + offset_y;
        if dx < dest.width() && dy < dest.height() {
            let base_pixel = dest.get_pixel_mut(dx, dy);
            blend_pixel(base_pixel, *overlay_pixel);
        }
    }
}

/// Holds a queue of operations to be performed. Useful for delaying some draw operations to ensure they are placed at the correct z-index.
impl DrawQueue {
    // Create a new, empty draw queue
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    // Add a new draw operation to the queue
    pub fn queue(&mut self, task: DrawTask) {
        self.queue.push(task);
    }

    // Execute all operations in the queue with a blank buffer
    pub fn execute(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(self.width(), self.height(), Rgba([0, 0, 0, 0]));

        self.execute_with_buffer(buffer)
    }

    // Execute all operations in the queue with a provided buffer
    pub fn execute_with_buffer(
        &self,
        mut buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
    ) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        // TODO - Assert the provided buffer size is sufficient

        for task in &self.queue {
            match task {
                DrawTask::FilledRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                } => {
                    // foo

                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    let rect = Rect::at(*x, *y).of_size(*width, *height);

                    draw_filled_rect_mut(&mut buffer, rect, color);
                }
                DrawTask::HollowRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                } => {
                    // foo

                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    let rect = Rect::at(*x, *y).of_size(*width, *height);

                    draw_hollow_rect_mut(&mut buffer, rect, color);
                }
                DrawTask::Text {
                    x,
                    y,
                    scale,
                    rgba,
                    text,
                } => {
                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    draw_text_mut(&mut buffer, color, *x, *y, *scale, &font, text);
                }
                DrawTask::Line { start, end, rgba } => {
                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    draw_line_segment_mut(&mut buffer, *start, *end, color);
                }
                DrawTask::Copy { draw_queue, x, y } => {
                    // buffer
                    //     .copy_from(&draw_queue.execute(), *x, *y)
                    //     .unwrap_or_else(|err| {
                    //         eprintln!("Unable to copy drawing: {:?}", err);
                    //         panic!("Unable to copy drawing");
                    //     });
                    blend_images(&mut buffer, &draw_queue.execute(), *x, *y);
                }
            }
        }

        buffer
    }
}
