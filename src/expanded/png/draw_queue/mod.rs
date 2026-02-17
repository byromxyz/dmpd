use std::collections::VecDeque;

use ab_glyph::FontRef;
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};
use log::warn;
use serde::Serialize;

use crate::util::{draw::text_dimensions, error::DrawError};

use once_cell::sync::Lazy;

pub static FONT: Lazy<FontRef<'static>> = Lazy::new(|| {
    FontRef::try_from_slice(include_bytes!("../../../fonts/NimbusSanL-Reg.otf"))
        .expect(&DrawError::CannotCreateFont.describe())
});

/// Define a structure to hold a queue of draw operations
#[derive(Debug, Clone, Serialize)]
pub struct DrawQueue {
    pub queue: VecDeque<DrawTask>,
    pub width: u32,
    pub height: u32,
}

impl DrawQueue {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
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
        hatch: Option<(u8, u8, u8, u8)>,
        radius: Option<(u32, u32, u32, u32)>,
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
        // text: Option<String>,
    },
    Copy {
        draw_queue: DrawQueue,
        x: u32,
        y: u32,
    },
}

impl DrawTask {
    fn end_x(&self) -> u32 {
        let x = match &self {
            DrawTask::Line { end, .. } => end.0 as u32,
            DrawTask::FilledRect { x, width, .. } => (x + *width as i32) as u32,
            DrawTask::HollowRect { x, width, .. } => (*x + *width as i32) as u32,
            DrawTask::Text { x, text, scale, .. } => {
                let (width, _) = text_dimensions(&*FONT, text, *scale);

                (*x + width as i32) as u32
            }
            DrawTask::Copy {
                draw_queue,
                x,
                y: _,
            } => *x + draw_queue.width(),
        };

        x
    }

    fn end_y(&self) -> u32 {
        let y = match &self {
            DrawTask::Line { end, .. } => end.1 as u32,
            DrawTask::FilledRect { y, height, .. } => (*y + *height as i32) as u32,
            DrawTask::HollowRect { y, height, .. } => (*y + *height as i32) as u32,
            DrawTask::Text { y, text, scale, .. } => {
                let (_, height) = text_dimensions(&*FONT, text, *scale);

                (*y + height as i32) as u32
            }
            DrawTask::Copy {
                draw_queue,
                x: _,
                y,
            } => *y + draw_queue.height(),
        };

        y
    }
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
        Self {
            queue: VecDeque::new(),
            width: 0,
            height: 0,
        }
    }

    // Add a new draw operation to the queue
    pub fn queue(&mut self, task: DrawTask) {
        let end_x: u32 = task.end_x();
        let end_y: u32 = task.end_y();

        self.width = self.width.max(end_x);
        self.height = self.height.max(end_y);

        self.queue.push_back(task);
    }

    // Add a new draw operation to the queue
    pub fn push(&mut self, task: DrawTask) {
        let end_x: u32 = task.end_x();
        let end_y: u32 = task.end_y();

        self.width = self.width.max(end_x);
        self.height = self.height.max(end_y);

        self.queue.push_front(task);
    }

    // Execute all operations in the queue with a blank buffer
    pub fn execute(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(self.width(), self.height(), Rgba([0, 0, 0, 0]));

        self.execute_with_buffer(buffer)
    }

    pub fn trim(
        &self,
        (x_min, y_min): (i32, i32), //
        (x_max, y_max): (i32, i32), //
    ) -> DrawQueue {
        let font = FontRef::try_from_slice(include_bytes!("../../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        let mut new_queue = DrawQueue::new();

        for task in &self.queue {
            match task {
                DrawTask::FilledRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                    hatch,
                    radius,
                } => {
                    if *y > y_max || *x > x_max {
                        continue;
                    } // If it begins after the bound, ignore
                    if *y + (*height as i32) < y_min || *x + (*width as i32) < x_min {
                        continue;
                    } // If it ends before the bound, ignore

                    let (x0, y0) = ((*x).max(x_min), (*y).max(y_min));
                    let (x1, y1) = (
                        (*x + *width as i32).min(x_max),
                        (*y + *height as i32).min(y_max),
                    );

                    // Track which edges were trimmed
                    let left_trimmed = x0 > *x;
                    let top_trimmed = y0 > *y;
                    let right_trimmed = x1 < *x + *width as i32;
                    let bottom_trimmed = y1 < *y + *height as i32;

                    let (x, y) = (x0, y0);
                    let width = (x1 - x0) as u32;

                    let height = (y1 - y0) as u32;

                    // Adjust radius for trimmed corners
                    let adjusted_radius = radius.map(|(r_tl, r_tr, r_bl, r_br)| {
                        let new_r_tl = if top_trimmed || left_trimmed { 0 } else { r_tl };
                        let new_r_tr = if top_trimmed || right_trimmed {
                            0
                        } else {
                            r_tr
                        };
                        let new_r_bl = if bottom_trimmed || left_trimmed {
                            0
                        } else {
                            r_bl
                        };
                        let new_r_br = if bottom_trimmed || right_trimmed {
                            0
                        } else {
                            r_br
                        };
                        (new_r_tl, new_r_tr, new_r_bl, new_r_br)
                    });

                    new_queue.queue(DrawTask::FilledRect {
                        x: x - x_min,
                        y: y - y_min,
                        width,
                        height,
                        rgba: *rgba,
                        hatch: *hatch,
                        radius: adjusted_radius,
                    });
                }
                DrawTask::HollowRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                } => {
                    if *y > y_max || *x > x_max {
                        continue;
                    } // If it begins after the bound, ignore
                    if *y + (*height as i32) < y_min || *x + (*width as i32) < x_min {
                        continue;
                    } // If it ends before the bound, ignore

                    let (x0, y0) = ((*x).max(x_min), (*y).max(y_min));
                    let (x1, y1) = (
                        (*x + *width as i32).min(x_max),
                        (*y + *height as i32).min(y_max),
                    );

                    let (x, y) = (x0, y0);

                    let width = (x1 - x0) as u32;
                    let height = (y1 - y0) as u32;

                    let x = x - x_min;
                    let y = y - y_min;

                    if x >= x_min {
                        // Left
                        new_queue.queue(DrawTask::Line {
                            start: (x as f32, y as f32),
                            end: (x as f32, y as f32 + height as f32),
                            rgba: *rgba,
                        });
                    }

                    if x + (width as i32 - 1) < x_max {
                        // Right
                        new_queue.queue(DrawTask::Line {
                            start: (x as f32 + width as f32 - 1.0, y as f32),
                            end: (x as f32 + width as f32 - 1.0, y as f32 + height as f32),
                            rgba: *rgba,
                        });
                    }

                    if y >= y_min {
                        // Top
                        new_queue.queue(DrawTask::Line {
                            start: (x as f32, y as f32),
                            end: (x as f32 + width as f32, y as f32),
                            rgba: *rgba,
                        });
                    }

                    if y + (height as i32 - 1) < y_max {
                        // Bottom
                        new_queue.queue(DrawTask::Line {
                            start: (x as f32, y as f32 + height as f32 - 1.0),
                            end: (x as f32 + width as f32, y as f32 + height as f32 - 1.0),
                            rgba: *rgba,
                        });
                    }
                }
                DrawTask::Text {
                    x,
                    y,
                    scale,
                    rgba,
                    text,
                } => {
                    let (width, height) = text_dimensions(&*FONT, text, *scale);

                    if *y < y_min || *x < x_min {
                        continue;
                    } // Do not draw if it starts before bounds
                    if *y + (height as i32) > y_max || *x + (width as i32) > x_max {
                        continue;
                    } // Do not draw if it draws out of bounds

                    new_queue.queue(DrawTask::Text {
                        x: x - x_min,
                        y: y - y_min,
                        scale: *scale,
                        rgba: *rgba,
                        text: text.clone(),
                    })
                }
                DrawTask::Line { start, end, rgba } => {
                    let (x0, y0) = *start;
                    let (x1, y1) = *end;

                    if y1 < y_min as f32 || x1 < x_min as f32 {
                        continue;
                    }
                    if y0 > y_max as f32 || x0 > x_max as f32 {
                        continue;
                    }

                    let start = (
                        x0.max(x_min as f32) - x_min as f32,
                        y0.max(y_min as f32) - y_min as f32,
                    );
                    let end = (
                        x1.min(x_max as f32) - x_min as f32,
                        y1.min(y_max as f32) - y_min as f32,
                    );

                    new_queue.queue(DrawTask::Line {
                        start,
                        end,
                        rgba: *rgba,
                    })
                }
                // DrawTask::Copy { draw_queue, x, y } => {
                //     if *y > y_max as u32 || *x > x_max as u32 {
                //         continue;
                //     }
                //     if y + draw_queue.height() < y_min as u32
                //         || x + draw_queue.width() < x_min as u32
                //     {
                //         continue;
                //     };

                //     let inner_queue = draw_queue.trim(
                //         (x_min - *x as i32, y_min - *y as i32),
                //         (x_max - *x as i32, y_max - *y as i32),
                //     );

                //     new_queue.queue(DrawTask::Copy {
                //         draw_queue: inner_queue,
                //         x: if *x < x_min as u32 {
                //             0 as u32
                //         } else {
                //             *x - x_min as u32
                //         },
                //         y: if *y < y_min as u32 {
                //             0 as u32
                //         } else {
                //             *y - y_min as u32
                //         },
                //     });
                // }
                DrawTask::Copy { draw_queue, x, y } => {
                    if *y > y_max as u32 || *x > x_max as u32 {
                        continue;
                    }
                    if y + draw_queue.height() < y_min as u32
                        || x + draw_queue.width() < x_min as u32
                    {
                        continue;
                    };

                    let inset_x = if (x_min as u32) < *x as u32 {
                        0u32
                    } else {
                        x_min as u32 - x
                    };

                    let inset_y = if (y_min as u32) < *y as u32 {
                        0u32
                    } else {
                        y_min as u32 - y
                    };

                    let cap_x = if (x_max as u32) < *x + draw_queue.width() as u32 {
                        x_max - *x as i32
                    } else {
                        draw_queue.width() as i32
                    };

                    let cap_y = if (y_max as u32) < *y + draw_queue.height() as u32 {
                        y_max - *y as i32
                    } else {
                        draw_queue.height() as i32
                    };

                    let inner_queue = draw_queue.trim(
                        (inset_x as i32, inset_y as i32),
                        (cap_x as i32, cap_y as i32),
                    );

                    new_queue.queue(DrawTask::Copy {
                        draw_queue: inner_queue,
                        x: if *x < x_min as u32 {
                            0 as u32
                        } else {
                            *x - x_min as u32
                        },
                        y: if *y < y_min as u32 {
                            0 as u32
                        } else {
                            *y - y_min as u32
                        },
                    });
                }
            }
        }

        return new_queue;
    }

    // Execute all operations in the queue with a provided buffer
    pub fn execute_with_buffer(
        &self,
        mut buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
    ) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        // TODO - Assert the provided buffer size is sufficient

        let font = FontRef::try_from_slice(include_bytes!("../../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

        for task in &self.queue {
            match task {
                DrawTask::FilledRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                    hatch,
                    radius,
                } => {
                    let (r, g, b, a) = rgba;
                    let color = Rgba([*r, *g, *b, *a]);

                    if *height == 0 {
                        //  TODO - Prevent this
                        warn!("Attempting to draw a rect with 0 height at ({}, {}) width: {} height: {}", *x, *y, *width, *height);
                    } else {
                        // Helper function to check if point is inside rounded rectangle
                        let is_inside_rounded_rect =
                            |px: u32, py: u32, radii: (u32, u32, u32, u32)| -> bool {
                                let px_i = px as i32;
                                let py_i = py as i32;
                                let (r_tl, r_tr, r_bl, r_br) = radii;

                                // Define corner regions
                                let left = *x;
                                let right = *x + *width as i32;
                                let top = *y;
                                let bottom = *y + *height as i32;

                                // Check if in corner region
                                let in_top_left =
                                    px_i < left + r_tl as i32 && py_i < top + r_tl as i32;
                                let in_top_right =
                                    px_i >= right - r_tr as i32 && py_i < top + r_tr as i32;
                                let in_bottom_left =
                                    px_i < left + r_bl as i32 && py_i >= bottom - r_bl as i32;
                                let in_bottom_right =
                                    px_i >= right - r_br as i32 && py_i >= bottom - r_br as i32;

                                if in_top_left {
                                    let dx = (px_i - (left + r_tl as i32)) as f32;
                                    let dy = (py_i - (top + r_tl as i32)) as f32;
                                    dx * dx + dy * dy <= (r_tl * r_tl) as f32
                                } else if in_top_right {
                                    let dx = (px_i - (right - r_tr as i32)) as f32;
                                    let dy = (py_i - (top + r_tr as i32)) as f32;
                                    dx * dx + dy * dy <= (r_tr * r_tr) as f32
                                } else if in_bottom_left {
                                    let dx = (px_i - (left + r_bl as i32)) as f32;
                                    let dy = (py_i - (bottom - r_bl as i32)) as f32;
                                    dx * dx + dy * dy <= (r_bl * r_bl) as f32
                                } else if in_bottom_right {
                                    let dx = (px_i - (right - r_br as i32)) as f32;
                                    let dy = (py_i - (bottom - r_br as i32)) as f32;
                                    dx * dx + dy * dy <= (r_br * r_br) as f32
                                } else {
                                    true // Not in a corner, always inside
                                }
                            };

                        // Blend alpha channels with destination buffer
                        for py in 0..*height {
                            for px in 0..*width {
                                let dx = (*x + px as i32) as u32;
                                let dy = (*y + py as i32) as u32;

                                if dx < buffer.width() && dy < buffer.height() {
                                    // Check if we should draw this pixel based on radius
                                    let should_draw = if let Some(radii) = radius {
                                        let (r_tl, r_tr, r_bl, r_br) = radii;
                                        if *r_tl > 0 || *r_tr > 0 || *r_bl > 0 || *r_br > 0 {
                                            is_inside_rounded_rect(dx, dy, *radii)
                                        } else {
                                            true
                                        }
                                    } else {
                                        true
                                    };

                                    if should_draw {
                                        let base_pixel = buffer.get_pixel_mut(dx, dy);
                                        blend_pixel(base_pixel, color);
                                    }
                                }
                            }
                        }

                        if let Some(hatch_color) = *hatch {
                            let (r, g, b, a) = hatch_color;
                            let color = Rgba([r, g, b, a]);
                            let gap = 4;

                            for py in 0..*height {
                                for px in 0..*width {
                                    let dx = (*x + px as i32) as u32;
                                    let dy = (*y + py as i32) as u32;

                                    if dx < buffer.width() && dy < buffer.height() {
                                        // Determine if this pixel should be hatched
                                        // Draw diagonal lines from bottom-left to top-right
                                        let should_hatch = (px as i32 + py as i32) % gap == 0;

                                        if should_hatch {
                                            // Check if hatch line point is inside rounded rect
                                            let should_draw = if let Some(radii) = radius {
                                                let (r_tl, r_tr, r_bl, r_br) = radii;
                                                if *r_tl > 0 || *r_tr > 0 || *r_bl > 0 || *r_br > 0
                                                {
                                                    is_inside_rounded_rect(dx, dy, *radii)
                                                } else {
                                                    true
                                                }
                                            } else {
                                                true
                                            };

                                            if should_draw {
                                                let base_pixel = buffer.get_pixel_mut(dx, dy);
                                                blend_pixel(base_pixel, color);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                DrawTask::HollowRect {
                    x,
                    y,
                    width,
                    height,
                    rgba,
                } => {
                    let (r, g, b, a) = rgba;
                    let color = Rgba([*r, *g, *b, *a]);

                    // Draw rectangle borders with alpha blending
                    // Top edge
                    for px in 0..*width {
                        let dx = (*x + px as i32) as u32;
                        let dy = *y as u32;
                        if dx < buffer.width() && dy < buffer.height() {
                            let base_pixel = buffer.get_pixel_mut(dx, dy);
                            blend_pixel(base_pixel, color);
                        }
                    }
                    // Bottom edge
                    for px in 0..*width {
                        let dx = (*x + px as i32) as u32;
                        let dy = (*y + *height as i32 - 1) as u32;
                        if dx < buffer.width() && dy < buffer.height() {
                            let base_pixel = buffer.get_pixel_mut(dx, dy);
                            blend_pixel(base_pixel, color);
                        }
                    }
                    // Left edge
                    for py in 0..*height {
                        let dx = *x as u32;
                        let dy = (*y + py as i32) as u32;
                        if dx < buffer.width() && dy < buffer.height() {
                            let base_pixel = buffer.get_pixel_mut(dx, dy);
                            blend_pixel(base_pixel, color);
                        }
                    }
                    // Right edge
                    for py in 0..*height {
                        let dx = (*x + *width as i32 - 1) as u32;
                        let dy = (*y + py as i32) as u32;
                        if dx < buffer.width() && dy < buffer.height() {
                            let base_pixel = buffer.get_pixel_mut(dx, dy);
                            blend_pixel(base_pixel, color);
                        }
                    }
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

                    // Draw text to a temporary buffer then blend it
                    let (text_width, text_height) = text_dimensions(&*FONT, text, *scale);
                    if text_width > 0 && text_height > 0 {
                        let mut temp_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
                            ImageBuffer::from_pixel(text_width, text_height, Rgba([0, 0, 0, 0]));
                        draw_text_mut(&mut temp_buffer, color, 0, 0, *scale, &font, text);
                        blend_images(&mut buffer, &temp_buffer, *x as u32, *y as u32);
                    }
                }
                DrawTask::Line { start, end, rgba } => {
                    let (r, g, b, a) = rgba;
                    let color = Rgba([*r, *g, *b, *a]);

                    // Draw line with alpha blending using Bresenham's algorithm
                    let (x0, y0) = (start.0 as i32, start.1 as i32);
                    let (x1, y1) = (end.0 as i32, end.1 as i32);

                    let dx = (x1 - x0).abs();
                    let dy = (y1 - y0).abs();
                    let sx = if x0 < x1 { 1 } else { -1 };
                    let sy = if y0 < y1 { 1 } else { -1 };
                    let mut err = dx - dy;
                    let mut x = x0;
                    let mut y = y0;

                    loop {
                        if x >= 0
                            && y >= 0
                            && (x as u32) < buffer.width()
                            && (y as u32) < buffer.height()
                        {
                            let base_pixel = buffer.get_pixel_mut(x as u32, y as u32);
                            blend_pixel(base_pixel, color);
                        }

                        if x == x1 && y == y1 {
                            break;
                        }

                        let e2 = 2 * err;
                        if e2 > -dy {
                            err -= dy;
                            x += sx;
                        }
                        if e2 < dx {
                            err += dx;
                            y += sy;
                        }
                    }
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

#[cfg(test)]
mod test;
