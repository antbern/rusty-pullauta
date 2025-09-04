use tiny_skia::{PathBuilder, Transform};

pub struct Canvas<'a> {
    pixmap: tiny_skia::Pixmap,
    ppaint: tiny_skia::Paint<'a>,
    stroke: tiny_skia::Stroke,
}

impl Canvas<'_> {
    pub fn new(width: i32, height: i32) -> Self {
        Self::from_pixmap(tiny_skia::Pixmap::new(width as u32, height as u32).unwrap())
    }

    fn from_pixmap(pixmap: tiny_skia::Pixmap) -> Self {
        let mut ppaint = tiny_skia::Paint::default();
        ppaint.set_color(tiny_skia::Color::BLACK);
        ppaint.anti_alias = false;

        let stroke = tiny_skia::Stroke {
            width: 1.0,
            ..Default::default()
        };

        Canvas {
            pixmap,
            ppaint,
            stroke,
        }
    }

    #[inline]
    pub fn set_line_width(&mut self, width: f32) {
        self.stroke.width = width;
    }

    #[inline]
    pub fn set_color(&mut self, rgb: (u8, u8, u8)) {
        self.ppaint
            .set_color(tiny_skia::Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255));
    }

    #[inline]
    pub fn set_transparent_color(&mut self) {
        self.ppaint.blend_mode = tiny_skia::BlendMode::SourceIn;
        self.ppaint
            .set_color(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
    }

    #[inline]
    pub fn set_stroke_cap_round(&mut self) {
        self.stroke.line_cap = tiny_skia::LineCap::Round;
    }

    #[inline]
    pub fn unset_stroke_cap(&mut self) {
        self.stroke.line_cap = tiny_skia::LineCap::Butt;
    }

    #[inline]
    pub fn set_dash(&mut self, interval_on: f32, interval_off: f32) {
        self.stroke.dash = tiny_skia::StrokeDash::new(vec![interval_on, interval_off], 0.0);
    }

    #[inline]
    pub fn unset_dash(&mut self) {
        self.stroke.dash = None;
    }

    #[inline]
    pub fn draw_polyline(&mut self, pts: &[(f32, f32)]) {
        let mut pb = PathBuilder::new();

        pb.move_to(pts[0].0, pts[0].1);
        for pt in pts.iter() {
            pb.line_to(pt.0, pt.1);
        }
        let path = pb.finish().unwrap();

        self.pixmap.stroke_path(
            &path,
            &self.ppaint,
            &self.stroke,
            Transform::identity(),
            None,
        );
    }

    #[inline]
    pub fn draw_closed_polyline(&mut self, pts: &[(f32, f32)]) {
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].0, pts[0].1);
        for pt in pts.iter() {
            pb.line_to(pt.0, pt.1);
        }
        let path = pb.finish().unwrap();

        self.pixmap.stroke_path(
            &path,
            &self.ppaint,
            &self.stroke,
            Transform::identity(),
            None,
        );
    }

    #[inline]
    pub fn draw_filled_polygon(&mut self, apts: &[Vec<(f32, f32)>]) {
        let mut pb = PathBuilder::new();
        for pts in apts {
            pb.move_to(pts[0].0, pts[0].1);
            for pt in pts.iter() {
                pb.line_to(pt.0, pt.1);
            }
        }
        let path = pb.finish().unwrap();

        self.stroke.width = 1.0;

        self.pixmap.fill_path(
            &path,
            &self.ppaint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );

        self.pixmap.stroke_path(
            &path,
            &self.ppaint,
            &self.stroke,
            Transform::identity(),
            None,
        );
    }

    #[inline]
    pub fn save_as(&mut self, filename: &std::path::Path) {
        self.pixmap.save_png(filename).unwrap()
    }

    #[inline]
    pub fn load_from(filename: &std::path::Path) -> Canvas {
        let pixmap = tiny_skia::Pixmap::load_png(filename).unwrap();
        Canvas::from_pixmap(pixmap)
    }

    #[inline]
    pub fn overlay(&mut self, other_canvas: &mut Canvas, x: f32, y: f32) {
        self.pixmap.draw_pixmap(
            x as i32,
            y as i32,
            other_canvas.pixmap.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }
}
