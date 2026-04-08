use chrono::Local;
use log::info;
use sparko_embedded_std::{SparkoEmbeddedStd, config::{Config, ConfigValue, TypedValue}, task::Task, tz::TimeZone};

use crate::{Feature, config::{FeatureDescriptor, SharedConfig}, sparko_esp32_std::{SparkoEsp32Std, SparkoEsp32StdInitializer}};


pub const CORE_FEATURE_NAME: &str = "core";
pub const SSID: &str = "ssid";
pub const WIFI_PASSWORD: &str = "wifi_password";
pub const MDNS_HOSTNAME: &str = "mdns_hostname";
pub const TIMEZONE: &str = "time_zone";

pub const SSID_LEN: usize = 32;
pub const PASSWORD_LEN: usize = 64;
pub const HOSTNAME_LEN: usize = 32;
pub const FQDN_LEN: usize = 64;

pub struct Core {
    // The core feature provides wifi and mDNS
}

impl Core {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {}) 
    }
}

impl Feature for Core {
    fn init(&self, init: &mut crate::sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> {
        let config = Config::builder()
            .with(SSID.to_string(), ConfigValue { value: TypedValue::String(SSID_LEN, None), required: true })?
            .with(WIFI_PASSWORD.to_string(), ConfigValue { value: TypedValue::String(PASSWORD_LEN, None), required: true })?
            .with(MDNS_HOSTNAME.to_string(), ConfigValue { value: TypedValue::String(HOSTNAME_LEN, None), required: true })?
            .with(TIMEZONE.to_string(), ConfigValue { value: TypedValue::TimeZone(TimeZone::Utc), required: true })?
            .build();


        Ok(FeatureDescriptor {
            name: CORE_FEATURE_NAME.to_string(),
            config,
        })
    }
    
    fn start(&self, sparko: &mut SparkoEsp32Std, initializer: &mut SparkoEsp32StdInitializer, config: &SharedConfig) -> anyhow::Result<()> {
        let resolve_task = ResolveTask::new(config)?;
        initializer.add_task(Box::new(resolve_task), "0 * * * * *")?;
        Ok(())
    }
}


pub struct ResolveTask {
}

impl Task for ResolveTask {
    fn run(&mut self, _sparko_cyd: &dyn SparkoEmbeddedStd) -> anyhow::Result<()> {
        
        log::info!("Top of loop");

        let datetime = Local::now();
        info!("Time now: {}", datetime.format("%Y-%m-%d %H:%M:%S"));


        let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        let heap_min = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
        log::info!("heap free={} min={}", heap_free, heap_min);
        
        // TODO: force a reset if we run low on heap
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Core"
    }
}

impl ResolveTask {
    pub fn new(_config: &SharedConfig) -> anyhow::Result<Self> {
        Ok(Self {
        })
    }
}