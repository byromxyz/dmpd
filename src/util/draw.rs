use ab_glyph::{point, Font, GlyphId, PxScale, ScaleFont};

/// Estimates the size of a given `text` string at `font_size` in `font``.
/// ImageProc does not expose the expected size of draw_text_mut so this function is copied from source
/// https://github.com/image-rs/imageproc/blob/master/src/drawing/text.rs#L10-L37

pub fn text_dimensions(font: &impl Font, text: &str, font_size: f32) -> (u32, u32) {
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
