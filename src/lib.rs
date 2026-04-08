use crate::config::{FeatureDescriptor, SharedConfig};



pub mod sparko_esp32_std;

mod config;
mod wifi;
mod http;
mod portal;
mod led;
mod mdns;
mod core;
pub mod dyndns2;



pub trait Feature {
    fn init(&self, init: &mut sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> ;
    fn start(&self, sparko: &mut sparko_esp32_std::SparkoEsp32Std, initializer: &mut sparko_esp32_std::SparkoEsp32StdInitializer, config: &SharedConfig) -> anyhow::Result<()>;
}

pub trait FeatureConfig {

}