use sparko_embedded_std::{config::Config, feature::FeatureDescriptor};



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



pub trait Feature {
    fn init(&self, init: &mut sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> ;
    fn start(&mut self, sparko: &mut sparko_esp32_std::SparkoEsp32Std, initializer: &mut sparko_esp32_std::SparkoEsp32StdInitializer, config: &Config) -> anyhow::Result<()>;
}

pub trait FeatureConfig {

}