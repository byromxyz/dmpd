use std::collections::VecDeque;

use image::{ImageBuffer, Rgba};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut},
    rect::Rect,
};

/// Define a type for a drawing callback function
pub struct DrawRectOperation {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    rgba: (u8, u8, u8, u8),
    filled: bool,
}

/// Define a structure to hold a queue of draw operations
pub struct DrawQueue {
    queue: VecDeque<DrawRectOperation>,
}

/// Holds a queue of operations to be performed. Useful for delaying some draw operations to ensure they are placed at the correct z-index.
impl DrawQueue {
    // Create a new, empty draw queue
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    // Add a new draw operation to the queue
    pub fn schedule(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        rgba: (u8, u8, u8, u8),
        filled: bool,
    ) {
        self.queue.push_back(DrawRectOperation {
            x,
            y,
            width,
            height,
            rgba,
            filled,
        });
    }

    // Execute all operations in the queue
    pub fn execute(&mut self, img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        for DrawRectOperation {
            x,
            y,
            width,
            height,
            rgba,
            filled,
        } in &self.queue
        {
            let (r, g, b, a) = rgba;

            let rect = Rect::at(*x, *y).of_size(*width, *height);

            let color = Rgba([*r, *g, *b, *a]);

            if *filled {
                draw_filled_rect_mut(img, rect, color);
            } else {
                draw_hollow_rect_mut(img, rect, color);
            }
        }

        self.queue.clear()
    }
}
