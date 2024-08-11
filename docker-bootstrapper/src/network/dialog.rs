use std::collections::HashMap;

use color_eyre::owo_colors::{OwoColorize, Rgb, Style};
use rand::{rngs::ThreadRng, Rng};

use crate::Container;

#[derive(Default)]
pub(super) struct Dialogger<'a> {
    dia_len: usize,
    current_id: Option<&'a Container>,
    styler: Styler<'a>,
}

impl<'a> Dialogger<'a> {
    pub fn with_id(self, id: &'a Container) -> Self {
        let dia_len = if self.current_id == Some(id) {
            self.dia_len + 1
        } else {
            0
        };

        Self {
            dia_len,
            current_id: id.into(),
            styler: self.styler,
        }
    }

    pub fn log(mut self, id: &'a Container, msg: &str) -> Self {
        match self.current_id {
            Some(x) if x == id => self.print_mid(msg),
            Some(_) => {
                self.print_end();
                self.print_start(id, msg)
            }
            None => self.print_start(id, msg),
        };
        self.with_id(id)
    }

    pub fn print_end(&mut self) {
        if self.dia_len > 0 {
            println!("{}", " ┗━━".style(self.current_style()));
        }
    }

    fn print_start(&mut self, id: &'a Container, msg: &str) {
        println!("{:<20}{}", id.name().style(self.get_style(id)), msg)
    }

    fn print_mid(&mut self, msg: &str) {
        println!("{:<20}{}", " ┃".style(self.current_style()), msg)
    }

    fn current_style(&mut self) -> Style {
        self.current_id
            .map(|id| self.styler.get(id.name()))
            .unwrap_or_default()
    }

    fn get_style(&mut self, id: &'a Container) -> Style {
        self.styler.get(id.name())
    }
}

#[derive(Default)]
struct Styler<'a> {
    rng: ThreadRng,
    colors: HashMap<&'a str, Style>,
}

impl<'a> Styler<'a> {
    fn get(&mut self, c: &'a str) -> Style {
        *self
            .colors
            .entry(c)
            .or_insert_with(|| Self::rand_style(&mut self.rng))
    }

    fn rand_style(rng: &mut ThreadRng) -> Style {
        let mix = (230, 190, 255);

        let r: u8 = rng.gen_range(0..=255);
        let g: u8 = rng.gen_range(0..=255);
        let b: u8 = rng.gen_range(0..=255);

        let r = r / 2 + mix.0 / 2;
        let g = g / 2 + mix.1 / 2;
        let b = b / 2 + mix.2 / 2;

        Style::new().color(Rgb(r, g, b)).bold()
    }
}
