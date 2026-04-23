use std::str::FromStr;
use std::net::IpAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::Mutex;

use esp_idf_svc::http::Method;
use esp_idf_svc::http::client::EspHttpConnection;
use log::info;
use sparko_embedded_std::SparkoEmbeddedStd;
use sparko_embedded_std::config::Config;
use sparko_embedded_std::config::ConfigSpec;
use sparko_embedded_std::config::ConfigSpecValue;
use sparko_embedded_std::config::TypedValue;
use sparko_embedded_std::graphics::ClockRenderer;
use sparko_embedded_std::graphics::DisplayManager;
use sparko_embedded_std::task::Task;

use crate::sparko_esp32_std::SparkoEsp32Std;
use crate::sparko_esp32_std::SparkoEsp32StdInitializer;
use crate::{Feature, FeatureDescriptor};

//                                           123456789012345<-------- Max Name Length 15
// pub const USER_NAME: &str =                 "user_name";
// pub const PASSWORD: &str =                  "password";
// pub const HOSTNAME: &str =                  "hostname";
// pub const BASE_SERVICE_URL: &str =          "base_url";
// pub const GET_IP_URL: &str =                "get_ip_url";
// pub const GET_REQUIRES_STRIP: &str =        "get_req_strip";
// pub const UPDATE_URL: &str =                "update_url";
// pub const UPDATE_REQUIRES_ADDRESS: &str =   "upd_req_addr";
// pub const UPDATE_INTERVAL: &str =           "upd_int";
// pub const SCHEDULE: &str =                  "schedule";

pub struct AnalogClock {
}

impl AnalogClock {


    pub fn new() -> anyhow::Result<Self> {
        
        Ok(Self {
        })
    }
}

impl Feature for AnalogClock {
    fn init(&self, _initializer: &mut crate::sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> {
        info!("AnalogClock::init()");
        let config = ConfigSpec::builder()
            // .with(USER_NAME.to_string(), ConfigSpecValue::new(TypedValue::String(32, None), true))?
            // .with(PASSWORD.to_string(), ConfigSpecValue::new(TypedValue::String(32, None), true))?
            // .with(HOSTNAME.to_string(), ConfigSpecValue::new(TypedValue::String(64, None), true))?
            // // .with(BASE_SERVICE_URL.to_string(), ConfigSpecValue::new(TypedValue::String(64, None), true))?
            // .with(GET_IP_URL.to_string(), ConfigSpecValue::new(TypedValue::String(64, None), true))?
            // // .with(GET_REQUIRES_STRIP.to_string(), ConfigSpecValue::new(TypedValue::Bool(false), false))?
            // .with(UPDATE_URL.to_string(), ConfigSpecValue::new(TypedValue::String(64, None), true))?
            // .with(UPDATE_REQUIRES_ADDRESS.to_string(), ConfigSpecValue::new(TypedValue::Bool(false), false ))?
            // .with(SCHEDULE.to_string(), ConfigSpecValue::new(TypedValue::Cron(None), true))?
            .build();
        
        Ok(FeatureDescriptor {
            name: "AnalogClock".to_string(),
            config,
        })
    }
    
    fn start(&mut self, sparko: &mut SparkoEsp32Std, initializer: &mut SparkoEsp32StdInitializer, config: &Config) -> anyhow::Result<()> {
        initializer.add_task(Box::new(ResolveTask{
            clock_renderer: ClockRenderer::new(&mut sparko.display_manager)?,
        }), "* * * * * *")?;
        Ok(())
    }

}

pub struct ResolveTask {
    clock_renderer: ClockRenderer,
}



impl<'a> Task<SparkoEsp32Std<'a>> for ResolveTask
{
    // fn run(&mut self, _sparko_cyd: &dyn SparkoEmbeddedStd) -> anyhow::Result<()> {
    //     let clock_renderer = 
    // }
    
    fn name(&self) -> &str {
        "Analog Clock"
    }
    
    fn run(&mut self, sparko_embedded: &mut SparkoEsp32Std<'a>) -> anyhow::Result<()> {
        self.clock_renderer.update(&mut sparko_embedded.display_manager)
    }
}