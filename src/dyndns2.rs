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
use sparko_embedded_std::config::ConfigValue;
use sparko_embedded_std::config::TypedValue;
use sparko_embedded_std::task::Task;

use crate::config::SharedConfig;
use crate::sparko_esp32_std::SparkoEsp32Std;
use crate::sparko_esp32_std::SparkoEsp32StdInitializer;
use crate::{Feature, FeatureDescriptor};

pub const USER_NAME: &str = "user_name";
pub const PASSWORD: &str = "password";
pub const HOSTNAME: &str = "hostname";
pub const BASE_SERVICE_URL: &str = "base_url";
pub const GET_IP_URL: &str = "get_ip_url";
pub const GET_REQUIRES_STRIP: &str = "get_req_strip";
pub const UPDATE_URL: &str = "update_url";
pub const UPDATE_REQUIRES_ADDRESS: &str = "upd_req_addr";
pub const UPDATE_INTERVAL: &str = "upd_int";

// pub struct DynDns2Config {
//     user_name: String,
//     password: String,
//     hostname: String,
//     base_service_url: String,
//     get_ip_url: Option<String>,
//     get_requires_strip: bool,
//     update_url: Option<String>,
//     update_requires_address: bool,
//     update_interval: u64,
// }

// impl DynDns2Config {
//     pub fn new(config_manager: &ConfigManager) -> anyhow::Result<Self> {
//         Ok(Self {
//             user_name: config_manager.get(USER_NAME)?.unwrap_or_default(),
//             password: config_manager.get(PASSWORD)?.unwrap_or_default(),
//             hostname: config_manager.get(HOSTNAME)?.unwrap_or_default(),
//             base_service_url: config_manager.get(BASE_SERVICE_URL)?.unwrap_or_default(),
//             get_ip_url: config_manager.get(GET_IP_URL)?,
//             get_requires_strip: config_manager.get(GET_REQUIRES_STRIP)?.unwrap_or(false),
//             update_url: config_manager.get(UPDATE_URL)?,
//             update_requires_address: config_manager.get(UPDATE_REQUIRES_ADDRESS)?.unwrap_or(false),
//             update_interval: config_manager.get(UPDATE_INTERVAL)?.unwrap_or(3600),
//         })
//     }
// }

pub struct DynDns2 {
}

impl DynDns2 {


    pub fn new() -> anyhow::Result<Self> {
        
        Ok(Self {
        })
    }
}

impl Feature for DynDns2 {
    fn init(&self, initializer: &mut crate::sparko_esp32_std::SparkoEsp32StdInitializer) -> anyhow::Result<FeatureDescriptor> {
        let config = Config::builder()
            .with(USER_NAME.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(PASSWORD.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(HOSTNAME.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(BASE_SERVICE_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(GET_IP_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(GET_REQUIRES_STRIP.to_string(), ConfigValue { value: TypedValue::Bool(false), required: true })?
            .with(UPDATE_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?
            .with(UPDATE_REQUIRES_ADDRESS.to_string(), ConfigValue { value: TypedValue::Bool(false), required: false })?
            .with(UPDATE_INTERVAL.to_string(), ConfigValue { value: TypedValue::Int64(Some(3600)), required: true })?
            .build();
        


    
    
        

        Ok(FeatureDescriptor {
            name: "DynDNS2".to_string(),
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
    host_name: String,
    addr: Arc<Mutex<IpAddr>>,
    cnt: u32,
}

impl Task for ResolveTask {
    fn run(&mut self, _sparko_cyd: &dyn SparkoEmbeddedStd) -> anyhow::Result<()> {
        
        if self.cnt < 3 {
            match Self::get_public_ip_address() {
                Ok(public_ip) => {
                    self.cnt = self.cnt + 1;
                    if public_ip != *self.addr.clone().lock().unwrap() {
                        log::info!("Public IP changed: {} -> {}", *self.addr.lock().unwrap(), public_ip);
                        // *self.addr.lock()? = public_ip;
                    } else {
                        log::info!("Public IP unchanged: {}", public_ip);
                    }
                },
                Err(e) => {
                    log::error!("Failed to get public IP address: {}", e);
                }
            }
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "DynDns2 Resolver"
    }
}

impl ResolveTask {
    pub fn new(config: &SharedConfig) -> anyhow::Result<Self> {
        log::info!("Trace 3");
        let host_name = config.get_valid(HOSTNAME)?;
        let current_dns = Self::resolve_single(&host_name)?;
        info!("Current DNS resolution for {}: {}", &host_name, current_dns);

        let addr = Arc::new(Mutex::new(current_dns));
        Ok(Self { 
            host_name: host_name.to_string(),
            addr,
            cnt: 0,
        })
    }

    fn resolve_single(name: &str) -> anyhow::Result<IpAddr> {
        let addr = (name, 0)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("DNS returned no addresses"))?;

        Ok(addr.ip())
    }

    fn get_public_ip_address() -> anyhow::Result<IpAddr> {
        // HTTP client
        // let connection = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration::default())?;
        // let mut client = embedded_svc::http::client::Client::wrap(connection);

        let mut client = embedded_svc::http::client::Client::wrap(EspHttpConnection::new(&esp_idf_svc::http::client::Configuration {
            // use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        })?);

        let url = "https://svc.joker.com/nic/myip";

        let request = client.request(Method::Get, url, &[])?;
        let mut response = request.submit()?;

        println!("Status: {}", response.status());

        let mut body = [0u8; 512];
        let bytes_read = response.read(&mut body)?;

        let addr_str = core::str::from_utf8(&body[..bytes_read]).unwrap_or("invalid utf8").trim();
        let addr: IpAddr = IpAddr::from_str(addr_str)?;

        println!(
            "Body: {}",
            addr_str
        );
        println!(
            "IP Address: {}",
            addr
        );
        Ok(addr)
    }
}