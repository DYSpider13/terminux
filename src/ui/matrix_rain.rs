use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, graphene};
use rand::Rng;
use std::cell::RefCell;

const TICK_MS: u32 = 80; // ~12 FPS
const FONT_SIZE: f64 = 13.0;
const CHAR_HEIGHT: f64 = 15.0;

/// Characters used for the rain: half-width katakana, digits, some Latin
fn rain_charset() -> Vec<char> {
    let mut chars: Vec<char> = Vec::new();
    // Half-width katakana U+FF66–U+FF9D
    for c in 0xFF66u32..=0xFF9Du32 {
        if let Some(ch) = char::from_u32(c) {
            chars.push(ch);
        }
    }
    // Digits 0–9
    for c in '0'..='9' {
        chars.push(c);
    }
    // Some Latin characters
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
        chars.push(c);
    }
    chars
}

#[derive(Clone, Debug)]
struct RainDrop {
    y: f64,
    speed: f64,
    length: usize,
    chars: Vec<char>,
}

impl RainDrop {
    fn new_random(max_height: f64, charset: &[char]) -> Self {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(5..25);
        let chars: Vec<char> = (0..length).map(|_| charset[rng.gen_range(0..charset.len())]).collect();
        RainDrop {
            y: rng.gen_range(-max_height..0.0),
            speed: rng.gen_range(1.0..4.0),
            length,
            chars,
        }
    }

    fn reset(&mut self, max_height: f64, charset: &[char]) {
        let mut rng = rand::thread_rng();
        self.length = rng.gen_range(5..25);
        self.chars = (0..self.length)
            .map(|_| charset[rng.gen_range(0..charset.len())])
            .collect();
        self.y = rng.gen_range(-max_height..(0.0 - CHAR_HEIGHT * 2.0));
        self.speed = rng.gen_range(1.0..4.0);
    }
}

#[derive(Debug)]
struct MatrixRainState {
    drops: Vec<RainDrop>,
    charset: Vec<char>,
    columns: usize,
}

impl MatrixRainState {
    fn new() -> Self {
        Self {
            drops: Vec::new(),
            charset: rain_charset(),
            columns: 0,
        }
    }

    fn ensure_columns(&mut self, width: f64, height: f64) {
        let col_width = FONT_SIZE * 0.8;
        let needed = (width / col_width).ceil() as usize;
        if needed != self.columns {
            self.columns = needed;
            self.drops.clear();
            for _ in 0..needed {
                self.drops.push(RainDrop::new_random(height, &self.charset));
            }
        }
    }

    fn tick(&mut self, height: f64) {
        let mut rng = rand::thread_rng();
        for drop in &mut self.drops {
            drop.y += drop.speed * CHAR_HEIGHT;

            // If the entire trail is off the bottom, reset
            let trail_top = drop.y - (drop.length as f64) * CHAR_HEIGHT;
            if trail_top > height {
                drop.reset(height, &self.charset);
            }

            // Randomly shuffle one character in the trail
            if !drop.chars.is_empty() {
                let idx = rng.gen_range(0..drop.chars.len());
                drop.chars[idx] = self.charset[rng.gen_range(0..self.charset.len())];
            }
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MatrixRain {
        pub(super) state: RefCell<MatrixRainState>,
        pub(super) tick_source: RefCell<Option<glib::SourceId>>,
    }

    impl Default for MatrixRain {
        fn default() -> Self {
            Self {
                state: RefCell::new(MatrixRainState::new()),
                tick_source: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MatrixRain {
        const NAME: &'static str = "MatrixRain";
        type Type = super::MatrixRain;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for MatrixRain {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_can_target(false);
            self.obj().set_can_focus(false);
            self.obj().set_overflow(gtk4::Overflow::Hidden);
        }

        fn dispose(&self) {
            self.stop_animation();
        }
    }

    impl WidgetImpl for MatrixRain {
        fn realize(&self) {
            self.parent_realize();
            self.start_animation();
        }

        fn unrealize(&self) {
            self.stop_animation();
            self.parent_unrealize();
        }

        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f64;
            let height = widget.height() as f64;

            if width <= 0.0 || height <= 0.0 {
                return;
            }

            let mut state = self.state.borrow_mut();
            state.ensure_columns(width, height);

            let cr = snapshot.append_cairo(&graphene::Rect::new(0.0, 0.0, width as f32, height as f32));

            cr.select_font_face("monospace", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Normal);
            cr.set_font_size(FONT_SIZE);

            let col_width = FONT_SIZE * 0.8;

            for (col_idx, drop) in state.drops.iter().enumerate() {
                let x = col_idx as f64 * col_width;

                for (char_idx, &ch) in drop.chars.iter().enumerate() {
                    let char_y = drop.y - (char_idx as f64) * CHAR_HEIGHT;

                    // Skip characters outside visible area
                    if char_y < -CHAR_HEIGHT || char_y > height + CHAR_HEIGHT {
                        continue;
                    }

                    let alpha = if char_idx == 0 {
                        // Head character: brightest
                        0.10
                    } else {
                        // Trail: fade out
                        let fade = 1.0 - (char_idx as f64 / drop.length as f64);
                        0.02 + 0.04 * fade
                    };

                    if char_idx == 0 {
                        // Head: bright green #00ff41
                        cr.set_source_rgba(0.0, 1.0, 0.255, alpha);
                    } else {
                        // Trail: standard green #00cc33
                        cr.set_source_rgba(0.0, 0.8, 0.2, alpha);
                    }

                    let text = ch.to_string();
                    cr.move_to(x, char_y);
                    let _ = cr.show_text(&text);
                }
            }
        }
    }

    impl MatrixRain {
        fn start_animation(&self) {
            if self.tick_source.borrow().is_some() {
                return;
            }

            let widget = self.obj().clone();
            let source = glib::timeout_add_local(
                std::time::Duration::from_millis(TICK_MS as u64),
                move || {
                    let height = widget.height() as f64;
                    if height > 0.0 {
                        let imp = widget.imp();
                        imp.state.borrow_mut().tick(height);
                        widget.queue_draw();
                    }
                    glib::ControlFlow::Continue
                },
            );
            self.tick_source.replace(Some(source));
        }

        fn stop_animation(&self) {
            if let Some(source) = self.tick_source.take() {
                source.remove();
            }
        }
    }
}

glib::wrapper! {
    pub struct MatrixRain(ObjectSubclass<imp::MatrixRain>)
        @extends gtk4::Widget;
}

impl MatrixRain {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
