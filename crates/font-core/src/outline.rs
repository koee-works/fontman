//! Glyph outlines and vertical metrics, for the outlines & metrics panel.
//!
//! The webview can draw text but can't say where a font's curves are, so the
//! outlines come from here: each glyph's path, its on-curve points and the
//! off-curve control points with the handles joining them, its advance and
//! side bearings, plus the face's vertical metrics. Everything is in font
//! units with y pointing up, as in the font; the panel flips and scales.
//!
//! Glyphs are looked up one character at a time through the cmap. No shaping
//! happens here, so ligatures and features don't show.

use serde::Serialize;

use crate::model::FaceRecord;
use crate::PreviewError;

/// The most characters drawn at once: enough for a word, few enough to stay
/// quick for fonts with heavy outlines.
pub const MAX_GLYPHS: usize = 32;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub x_height: Option<i16>,
    pub cap_height: Option<i16>,
    /// The two heights were measured from "x" and "H" because the font
    /// doesn't state them (old OS/2 tables).
    pub measured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphOutline {
    pub ch: char,
    /// None when the font has no glyph for the character.
    pub glyph_id: Option<u16>,
    pub name: Option<String>,
    pub advance: u16,
    pub lsb: i16,
    /// xMin, yMin, xMax, yMax; None for empty glyphs such as the space.
    pub bbox: Option<[i16; 4]>,
    /// SVG path data in font units, y up.
    pub d: String,
    /// Points the outline passes through.
    pub on_curve: Vec<[f32; 2]>,
    /// Control points the curve is pulled toward.
    pub off_curve: Vec<[f32; 2]>,
    /// Lines from each control point to the point it belongs to.
    pub handles: Vec<[f32; 4]>,
    pub contours: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outlines {
    pub metrics: Metrics,
    pub glyphs: Vec<GlyphOutline>,
}

/// Outlines of the first `MAX_GLYPHS` characters of `text`, and the face's
/// metrics, read from its file.
pub fn outlines(face: &FaceRecord, text: &str) -> Result<Outlines, PreviewError> {
    let data = std::fs::read(&face.file_path)?;
    let parsed =
        ttf_parser::Face::parse(&data, face.face_index).map_err(|_| PreviewError::Unsupported)?;
    Ok(outlines_of(&parsed, text))
}

pub fn outlines_of(face: &ttf_parser::Face, text: &str) -> Outlines {
    let glyphs = text.chars().take(MAX_GLYPHS).map(|ch| glyph(face, ch)).collect();
    Outlines { metrics: metrics(face), glyphs }
}

fn metrics(face: &ttf_parser::Face) -> Metrics {
    let top = |c: char| {
        face.glyph_index(c).and_then(|g| face.glyph_bounding_box(g)).map(|r| r.y_max)
    };
    // OS/2 version 2 and later state both; zero means "not set" in practice.
    let x = face.x_height().filter(|&v| v > 0);
    let cap = face.capital_height().filter(|&v| v > 0);
    let measured = x.is_none() || cap.is_none();
    Metrics {
        units_per_em: face.units_per_em(),
        ascender: face.ascender(),
        descender: face.descender(),
        line_gap: face.line_gap(),
        x_height: x.or_else(|| top('x')),
        cap_height: cap.or_else(|| top('H')),
        measured,
    }
}

fn glyph(face: &ttf_parser::Face, ch: char) -> GlyphOutline {
    let Some(id) = face.glyph_index(ch) else {
        return GlyphOutline {
            ch,
            glyph_id: None,
            name: None,
            advance: 0,
            lsb: 0,
            bbox: None,
            d: String::new(),
            on_curve: vec![],
            off_curve: vec![],
            handles: vec![],
            contours: 0,
        };
    };
    let mut b = Builder::default();
    let bbox = face.outline_glyph(id, &mut b).map(|r| [r.x_min, r.y_min, r.x_max, r.y_max]);
    GlyphOutline {
        ch,
        glyph_id: Some(id.0),
        name: face.glyph_name(id).map(str::to_owned),
        advance: face.glyph_hor_advance(id).unwrap_or(0),
        lsb: face.glyph_hor_side_bearing(id).unwrap_or(0),
        bbox,
        d: b.d,
        on_curve: b.on,
        off_curve: b.off,
        handles: b.handles,
        contours: b.contours,
    }
}

/// Collects an outline as SVG path data and as points and handles.
#[derive(Default)]
struct Builder {
    d: String,
    on: Vec<[f32; 2]>,
    off: Vec<[f32; 2]>,
    handles: Vec<[f32; 4]>,
    last: [f32; 2],
    /// Index in `on` where the current contour starts.
    start: usize,
    contours: u16,
}

/// Font units are integers, or close to it after variation; two decimals is
/// far below anything visible.
fn n(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r}") }
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.d += &format!("M{} {}", n(x), n(y));
        self.start = self.on.len();
        self.on.push([x, y]);
        self.last = [x, y];
        self.contours += 1;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.d += &format!("L{} {}", n(x), n(y));
        self.on.push([x, y]);
        self.last = [x, y];
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.d += &format!("Q{} {} {} {}", n(x1), n(y1), n(x), n(y));
        self.off.push([x1, y1]);
        self.handles.push([self.last[0], self.last[1], x1, y1]);
        self.handles.push([x1, y1, x, y]);
        self.on.push([x, y]);
        self.last = [x, y];
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.d += &format!("C{} {} {} {} {} {}", n(x1), n(y1), n(x2), n(y2), n(x), n(y));
        self.off.push([x1, y1]);
        self.off.push([x2, y2]);
        self.handles.push([self.last[0], self.last[1], x1, y1]);
        self.handles.push([x2, y2, x, y]);
        self.on.push([x, y]);
        self.last = [x, y];
    }
    fn close(&mut self) {
        self.d.push('Z');
        // A contour often ends with a segment back to where it began; that
        // point is already listed.
        if self.on.len() > self.start + 1 && self.on.last() == self.on.get(self.start) {
            self.on.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttf_parser::OutlineBuilder;

    #[test]
    fn builds_path_points_and_handles() {
        let mut b = Builder::default();
        b.move_to(0.0, 0.0);
        b.line_to(100.0, 0.0);
        b.quad_to(150.0, 50.0, 100.0, 100.0);
        b.curve_to(80.0, 120.0, 20.0, 120.0, 0.5, 100.25);
        b.line_to(0.0, 0.0);
        b.close();
        assert_eq!(b.d, "M0 0L100 0Q150 50 100 100C80 120 20 120 0.5 100.25L0 0Z");
        assert_eq!(b.on, vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.5, 100.25]]);
        assert_eq!(b.off, vec![[150.0, 50.0], [80.0, 120.0], [20.0, 120.0]]);
        // Quadratic: one control, joined to both ends. Cubic: each control to its end.
        assert_eq!(b.handles.len(), 4);
        assert_eq!(b.handles[0], [100.0, 0.0, 150.0, 50.0]);
        assert_eq!(b.handles[3], [20.0, 120.0, 0.5, 100.25]);
        assert_eq!(b.contours, 1);
    }

    #[test]
    fn numbers_print_compactly() {
        assert_eq!(n(12.0), "12");
        assert_eq!(n(-3.0), "-3");
        assert_eq!(n(1.004), "1");
        assert_eq!(n(-0.25), "-0.25");
        assert_eq!(n(2.5), "2.5");
    }
}
