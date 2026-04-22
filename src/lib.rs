
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
};
use sparko_embedded_std::{Color, config::Config, feature::FeatureDescriptor};



pub mod sparko_esp32_std;

mod config_store;
mod wifi;
mod http;
mod commands;
mod portal;
mod led;
mod mdns;
mod core;
pub mod dyndns2;
mod display_mipidsi;



pub trait Feature {
    fn init(&self, init: &mut sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> ;
    fn start(&mut self, sparko: &mut sparko_esp32_std::SparkoEsp32Std, initializer: &mut sparko_esp32_std::SparkoEsp32StdInitializer, config: &Config) -> anyhow::Result<()>;
}

pub trait FeatureConfig {

}

pub fn to_rgb565(color: Color) -> Rgb565 {
    match color {
        Color::Black => Rgb565::BLACK,
        Color::Red => Rgb565::RED,
        Color::Green => Rgb565::GREEN,
        Color::Blue => Rgb565::BLUE,
        Color::Yellow => Rgb565::YELLOW,
        Color::Magenta => Rgb565::MAGENTA,
        Color::Cyan => Rgb565::CYAN,
        Color::White => Rgb565::WHITE,
    }
}