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

/// Define a structure to hold a queue of draw operations
#[derive(Debug, Clone, Serialize)]
pub struct DrawQueue {
    queue: VecDeque<DrawTask>,
}

impl DrawQueue {
    pub fn width(&self) -> u32 {
        let font = FontRef::try_from_slice(include_bytes!("../../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

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
                DrawTask::Text { x, text, scale, .. } => {
                    let (width, _) = text_dimensions(&font, text, *scale);

                    return (*x + width as i32) as u32;
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
        let font = FontRef::try_from_slice(include_bytes!("../../../fonts/NimbusSanL-Reg.otf"))
            .expect(&DrawError::CannotCreateFont.describe());

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
                DrawTask::Text { y, text, scale, .. } => {
                    let (_, height) = text_dimensions(&font, text, *scale);

                    return (*y + height as i32) as u32;
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
        hatch: Option<(u8, u8, u8, u8)>,
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
        Self {
            queue: VecDeque::new(),
        }
    }

    // Add a new draw operation to the queue
    pub fn queue(&mut self, task: DrawTask) {
        self.queue.push_back(task);
    }

    // Add a new draw operation to the queue
    pub fn push(&mut self, task: DrawTask) {
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

                    new_queue.queue(DrawTask::FilledRect {
                        x: x - x_min,
                        y: y - y_min,
                        width,
                        height,
                        rgba: *rgba,
                        hatch: *hatch,
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
                    let (width, height) = text_dimensions(&font, text, *scale);

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
                } => {
                    // foo

                    let (r, g, b, a) = rgba;

                    let color = Rgba([*r, *g, *b, *a]);

                    if *height == 0 {
                        //  TODO - Prevent this
                        warn!("Attempting to draw a rect with 0 height at ({}, {}) width: {} height: {}", *x, *y, *width, *height);
                    } else {
                        let rect = Rect::at(*x, *y).of_size(*width, *height);

                        draw_filled_rect_mut(&mut buffer, rect, color);

                        if let Some(hatch_color) = *hatch {
                            for i in (0..(*width + *height)).step_by(10) {
                                let x_start = *x;
                                let y_start = *y + i as i32;
                                let x_end = *x + i as i32;
                                let y_end = *y;

                                if x_start <= *x + *width as i32 && y_end <= *y + *height as i32 {
                                    draw_line_segment_mut(
                                        &mut buffer,
                                        (x_start as f32, y_start as f32),
                                        (x_end as f32, y_end as f32),
                                        Rgba(hatch_color.into()),
                                    );
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

#[cfg(test)]
mod test;
