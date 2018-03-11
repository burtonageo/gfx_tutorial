use ::Styling;
use gfx::{Factory, Resources};
use gfx_glyph::{Scale, Section, GlyphBrush};

#[derive(Debug)]
pub struct FpsCounter {
    show_fps: bool,
    message_buf: String,
}

impl FpsCounter {
    #[inline]
    pub fn new() -> Self {
        FpsCounter {
            show_fps: true,
            message_buf: String::with_capacity(12),
        }
    }

    #[inline]
    pub fn toggle_show_fps(&mut self) {
        self.show_fps = !self.show_fps;
    }

    pub fn update_fps(&mut self, dt_s: f32) {
        use std::fmt::Write;
        self.message_buf.clear();
        if dt_s != 0.0f32 {
            write!(self.message_buf, "fps: {:.*}", 2, 1.0 / dt_s).unwrap();
        } else {
            self.message_buf.write_str("fps: N/A").unwrap();
        }
    }

    pub fn queue_text<R, F>(
        &self,
        styling: &Styling,
        brush: &mut GlyphBrush<R, F>)
    where
        R: Resources,
        F: Factory<R>,
    {
        let section = Section {
            text: &self.message_buf,
            screen_position: (5.0, 5.0),
            scale: Scale::uniform(32.0f32 * 2.0),
            color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        };
        if self.show_fps {
            //brush.queue(styling.to_section("Hello, world!"));
            brush.queue(section);
        }
    }
}