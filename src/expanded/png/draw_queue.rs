use ab_glyph::FontRef;
use image::{ImageBuffer, Rgba};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};

use crate::util::error::DrawError;

/// Define a structure to hold a queue of draw operations
pub struct DrawQueue {
    queue: Vec<DrawTask>,
}

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

    // Execute all operations in the queue
    pub fn execute(&mut self, img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        let font = FontRef::try_from_slice(include_bytes!("../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

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

                    draw_filled_rect_mut(img, rect, color);
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

                    draw_hollow_rect_mut(img, rect, color);
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

                    draw_text_mut(img, color, *x, *y, *scale, &font, text);
                }
                DrawTask::Line { start, end, rgba } => {
                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    draw_line_segment_mut(img, *start, *end, color);
                }
            }
        }

        self.queue.clear()
    }
}
