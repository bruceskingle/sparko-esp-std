use std::net::Ipv4Addr;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::{net::UdpSocket, sync::{Arc, Mutex}, thread};

use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, http::{Method}, nvs::EspDefaultNvsPartition};
use log::{error, info};
use sparko_embedded_std::config::Config;
use sparko_embedded_std::config_manager::{ConfigManager, ConfigManagerBuilder};
use sparko_embedded_std::{SparkoEmbeddedStd, problem::ProblemManager, task::{Task, TaskManager, TaskManagerBuilder}};
use sparko_embedded_std::http_server::HttpServerManager;
use esp_idf_svc::sntp::*;
use chrono::{Local, Utc};

use crate::Feature;
use crate::commands::EspCommands;
use crate::config_store::EspConfigStoreFactory;
use crate::http::EspHttpServerManager;
#[cfg(feature = "mono-led")]
use crate::led::MonoLedManager;
#[cfg(feature = "simple-led")]
use crate::led::SimpleLedManager;
#[cfg(feature = "rgb-led")]
use crate::led::RgbLedManager;

use crate::{core::Core, led::LedManager, wifi::WiFiManager};

use esp_idf_sys::*;
use std::ffi::CStr;

fn list_nvs_keys() {
    info!("Listing NVS keys:");
    unsafe {
    let mut it: nvs_iterator_t = std::ptr::null_mut();
    let part = CStr::from_bytes_with_nul_unchecked(b"nvs\0");
   

    let res = nvs_entry_find(
        part.as_ptr(), // partition name 
        // std::ptr::null(), // partition
        std::ptr::null(), // namespace
        nvs_type_t_NVS_TYPE_ANY,
        &mut it,
    );

    if res == ESP_OK {
        info!("NVS keys found:");
        while !it.is_null() {
            let mut info: nvs_entry_info_t = std::mem::zeroed();

            nvs_entry_info(it, &mut info);

            

            let namespace = CStr::from_ptr(info.namespace_name.as_ptr())
                .to_str()
                .unwrap();

            let key = CStr::from_ptr(info.key.as_ptr())
                .to_str()
                .unwrap();

            info!("NS: {}, Key: {}", namespace, key);

            nvs_entry_next(&mut it);
        }

        nvs_release_iterator(it);
    }
    else {
        info!("Failed to list NVS keys: {}", res);
    }
    info!("Finished listing NVS keys");
}
}

// #[derive(Eq, Hash, PartialEq)]
// struct Endpoint {
//     uri: String,
//     method: Method,
// }

// type PageHandler = Box<dyn for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> + Send + 'static>;


pub struct SparkoEsp32StdInitializer {
    task_manager_builder: TaskManagerBuilder,
}

impl SparkoEsp32StdInitializer {
    fn new() -> Self {
        Self {
            task_manager_builder: TaskManager::builder(),
        }
    }

    pub fn add_task(&mut self, task_initializer: Box<dyn Task>, schedule_spec: &str) -> anyhow::Result<()> {
        self.task_manager_builder.add_task(task_initializer, schedule_spec)?;
        Ok(())
    }

    // pub fn build(mut self) -> anyhow::Result<SparkoEsp32Std> {
    //     self.features.shrink_to_fit();
    //     SparkoEsp32Std::new(self.features, self.task_manager_builder.build())
    // }
}


pub struct SparkoEsp32StdBuilder {
    nvs_partition: esp_idf_svc::nvs::EspNvsPartition<esp_idf_svc::nvs::NvsDefault>,
    // failure_reason: Arc<Mutex<Option<String>>>,
    problem_manager: Arc<ProblemManager>,
    ap_mode: Arc<Mutex<bool>>,
    config_manager_builder: ConfigManagerBuilder,
    features: Vec::<FeatureHolder>,
    initializer: SparkoEsp32StdInitializer,

    core_feature: Core,
    core_feature_name: String,
    core_config_valid: bool,
    core_feature_config: Config,
    wifi_sender: Sender<Ipv4Addr>,
}

impl SparkoEsp32StdBuilder {
    fn new() -> anyhow::Result<Self> {
        // It is necessary to call this function once. Otherwise, some patches to the runtime
        // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
        esp_idf_svc::sys::link_patches();

        // Bind the log crate to the ESP Logging facilities
        esp_idf_svc::log::EspLogger::initialize_default();

        let nvs_partition: esp_idf_svc::nvs::EspNvsPartition<esp_idf_svc::nvs::NvsDefault> = EspDefaultNvsPartition::take()?;
        // let failure_reason: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let problem_manager = ProblemManager::new();
        let ap_mode = Arc::new(Mutex::new(false));

        list_nvs_keys();

        let config_store_factory = EspConfigStoreFactory::new(nvs_partition.clone(), problem_manager.clone())?;
        let mut config_manager_builder =  ConfigManager::builder(
            Box::new(config_store_factory), 
            problem_manager.clone(), 
            ap_mode.clone(),
            Box::new(EspCommands {}),
        )?;

        let mut initializer = SparkoEsp32StdInitializer::new();
        let (wifi_sender, wifi_receiver): (Sender<std::net::Ipv4Addr>, Receiver<std::net::Ipv4Addr>) = mpsc::channel();
        let core_feature = Core::new(wifi_receiver)?;
        let descriptor = core_feature.init(&mut initializer)?;
        let core_feature_name= descriptor.name.clone();
        let (core_feature_config, core_config_valid) = config_manager_builder.add_feature(descriptor, true)?;

        let builder = Self {
            nvs_partition,
            // failure_reason,
            problem_manager,
            features: Vec::new(),
            initializer,
            config_manager_builder,
            ap_mode,
            core_feature,
            core_feature_name,
            core_config_valid,
            core_feature_config,
            wifi_sender,
        };

        // builder.internal_add_feature(Box::new(Core::new()?), true)?;


        Ok(builder)
    }

    pub fn with_feature(mut self, feature: Box<dyn Feature>) -> anyhow::Result<Self> {
        self.internal_add_feature(feature, false)?;
        Ok(self)
    }

    fn internal_add_feature(&mut self, feature: Box<dyn Feature>, internal: bool) -> anyhow::Result<()> {
        // let descriptor = feature.init(&mut self.initializer)?;
        // self.features.push(feature);

        let descriptor = feature.init(&mut self.initializer)?;
        let name = descriptor.name.clone();
        let (config, _valid) = self.config_manager_builder.add_feature(descriptor, internal)?;
        self.features.push(FeatureHolder {
            feature,
            config,
            name,
        });

        Ok(())
    }

    pub fn build(mut self) -> anyhow::Result<SparkoEsp32StdRunner> {
        self.features.shrink_to_fit();
        
        let peripherals = Peripherals::take()?;


#[cfg(feature = "rgb-led")]
        let led_manager = RgbLedManager::new(true, 32, peripherals.ledc.timer0, 
            peripherals.ledc.channel0, peripherals.pins.gpio4, 
            peripherals.ledc.channel1, peripherals.pins.gpio16,
            peripherals.ledc.channel2, peripherals.pins.gpio17)?;
        

#[cfg(feature = "board-xiao-esp32c6")]
        let led_manager = MonoLedManager::new(true,  peripherals.pins.gpio15)?;
#[cfg(feature = "board-devkitv1")]
        let led_manager = MonoLedManager::new(false,  peripherals.pins.gpio2)?;

        led_manager.set_led_initializing()?;

        let sys_loop = EspSystemEventLoop::take()?;
        // let timer_service = EspTaskTimerService::new()?;



        let wifi_manager = //wifi::wifi(peripherals.modem, sys_loop,Some(nvs_partition.clone()),timer_service)?;
            WiFiManager::new(peripherals.modem,
                sys_loop,
                self.nvs_partition.clone(),
                &self.problem_manager,
                self.wifi_sender,
            )?;
        
        let bare_config_manager = //ConfigManager::new(nvs_partition, failure_reason, ap_mode.clone())?;
            self.config_manager_builder.build();

        let mut server_manager = EspHttpServerManager::new()?;

        server_manager.init_common_pages()?;
        server_manager.init_captive_portal(&self.ap_mode)?;
        
        let config_manager = Arc::new(bare_config_manager);
        ConfigManager::create_pages(&config_manager, &mut server_manager)?;

        // This should be in the app

    let cloned_ap_mode = self.ap_mode.clone();
    server_manager.on("/", Method::Get, move |req| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            if cloned_ap_mode.lock().unwrap().clone() {
                req.into_response(
                    302,
                    Some("Found"),
                    &[("Location", "/config")],
                )?;
            }
            else {

                let mut resp = req.into_ok_response()?;
                resp.write(r#"
                    <!DOCTYPE html>
                    <html lang="en">
                    <head>
                        <meta charset="utf-8" />
                        <meta name="viewport" content="width=device-width, initial-scale=1" />
                        <title>ESP32 Home</title>
                        <link rel="stylesheet" href="/main.css">
                    </head>
                    <body>
                        <div class="page">
                            <h1>ESP32 Home</h1>
                            <p>Welcome to the ESP32 home page!</p>
                            <p>Current time: "#.as_bytes())?;

                let now = Local::now();
                let time = now.format("%Y-%m-%d %H:%M:%S").to_string();
                resp.write(time.as_bytes())?;
                resp.write(r#"</p>
                        </div>
                    </body>
                    </html>
                    "#.as_bytes())?;
            }
            Ok(())
        })?;

        // END APP CODE

        Ok(SparkoEsp32StdRunner{
            sparko_std: SparkoEsp32Std {
                wifi_manager,
                led_manager,
                config_manager,
                server_manager,
                features: self.features,
                ap_mode: self.ap_mode,
                core_config_valid: self.core_config_valid,
            },
            initializer: self.initializer,
            core_feature_holder: FeatureHolder {
                feature: Box::new(self.core_feature),
                config: self.core_feature_config,
                name: self.core_feature_name,
            }
    })
    }
}

struct FeatureHolder {
    feature: Box<dyn Feature>,
    config: Config,
    name: String,
}


pub struct SparkoEsp32StdRunner {
    sparko_std: SparkoEsp32Std,
    initializer: SparkoEsp32StdInitializer,
    core_feature_holder: FeatureHolder,
}

impl SparkoEsp32StdRunner {
    pub fn start(mut self) -> anyhow::Result<()> {
        self.sparko_std.start(self.initializer, self.core_feature_holder)
    }
}


pub struct SparkoEsp32Std {
    pub wifi_manager: WiFiManager<'static>,
#[cfg(feature = "rgb-led")]
    pub led_manager: RgbLedManager<'static>,
#[cfg(feature = "mono-led")]
    pub led_manager: MonoLedManager,
#[cfg(feature = "simple-led")]
    pub led_manager: SimpleLedManager<'static>,
    pub config_manager: Arc<ConfigManager>,
    pub server_manager: EspHttpServerManager<'static>,
    features: Vec<FeatureHolder>,
    pub ap_mode: Arc<Mutex<bool>>,
    core_config_valid: bool,
}

impl SparkoEmbeddedStd for SparkoEsp32Std {
    
}

impl SparkoEsp32Std {
    pub fn builder() -> anyhow::Result<SparkoEsp32StdBuilder> {
        SparkoEsp32StdBuilder::new()
    }

    fn start_feature(&mut self, mut feature_holder: FeatureHolder, mut initializer: &mut SparkoEsp32StdInitializer) {
        if feature_holder.config.enabled.is_enabled() {
            match feature_holder.feature.start( self, &mut initializer, &feature_holder.config) {
                Ok(_) => info!("Started Feature {}", feature_holder.name),
                Err(error) => error!("FAILED to start Feature {}: {}", feature_holder.name, error),
            }
        }
        else {
            info!("Feature {} is disabled", feature_holder.name)
        }

        self.features.push(feature_holder);
    }

    fn start_client(&mut self, mut initializer: SparkoEsp32StdInitializer, core_feature_holder: FeatureHolder) -> anyhow::Result<()> {

        // start wifi

        let ip_address = self.wifi_manager.start_client(
                &core_feature_holder.config,
            )?;
        info!("Wifi started: ip_address={}", &ip_address);

        let sntp = EspSntp::new_default()?;
        
        info!("SNTP started, waiting for time sync...");

        loop {
            if let SyncStatus::Completed = sntp.get_sync_status() {
                break
            }
            info!("still waiting for time sync...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // self.sntp = Some(sntp); dont need this
        // std::thread::sleep(std::time::Duration::from_secs(2));

        let datetime = Utc::now();
        info!("Time synced: {}", datetime.format("%Y-%m-%d %H:%M:%S"));


        let features = std::mem::take(&mut self.features);
        self.features = Vec::with_capacity(features.len() + 1);

        self.start_feature(core_feature_holder, &mut initializer);

        for feature_holder in features {
            self.start_feature(feature_holder, &mut initializer);
        }

        let mut task_manager = initializer.task_manager_builder.build();

        self.led_manager.set_led_running()?;

        // This should never return
        task_manager.run(self)
    }
    

    fn start(&mut self, initializer: SparkoEsp32StdInitializer, core_feature_holder: FeatureHolder) -> anyhow::Result<()> {
        log::info!("sparko_cyd: top of run");
        if self.core_config_valid {
            log::info!("Loaded config");

            if let Err(error) = self.start_client(initializer, core_feature_holder) {
                log::error!("Error starting client: {}", error);
                self.led_manager.set_led_error()?;
            }
            else {
                log::info!("Client mode started successfully");
                return Ok(());
            }
        }
        else {
            self.led_manager.set_led_admin()?;
            info!("Invalid config, starting AP mode");
        }

        *self.ap_mode.lock().unwrap() = true;

        let server_addr = self.wifi_manager.start_access_point()?;

        thread::spawn(move || Self::captive_dns_server(server_addr));
        
        loop {
            log::info!("Top of AP loop");
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        // Self::system_halt("AP Loop terminated");
    }

    // fn system_halt<S: AsRef<str>>(s: S) {
    //     // TODO: Implement BSOD or similar system halt mechanism here
    //     println!("{}", s.as_ref());

    //     let bt = Backtrace::force_capture();
    //     println!("Stack trace:\n{bt}");

    //     std::process::exit(1);
    // }

    fn captive_dns_server(server_addr: std::net::Ipv4Addr)  {
        info!("DNS server start");
        let socket = UdpSocket::bind("0.0.0.0:53").unwrap();
        let addr_bytes = server_addr.octets();
        loop {
            let mut buf = [0u8; 512];

            // info!("DNS server recv_from...");
            let (size, src) = socket.recv_from(&mut buf).unwrap();

            // info!("DNS server recv_from...{:?}", &buf[..size]);

            let response = Self::build_dns_response(&buf[..size], &addr_bytes);

            socket.send_to(&response, src).unwrap();
        }
    }

    fn build_dns_response(query: &[u8], server_addr: &[u8; 4]) -> Vec<u8> {
        // info!("Received DNS query: {:?}", query);
        let mut resp = query.to_vec();

        resp[2] |= 0x80; // set QR bit (response)
        resp[3] |= 0x80; // set RD bit (recursion desired, optional)

        // Set ANCOUNT to 1 (answer count)
        resp[6] = 0x00;
        resp[7] = 0x01;

        resp.extend_from_slice(&[
            0xc0, 0x0c, // pointer to domain
            0x00, 0x01, // type A
            0x00, 0x01, // class IN
            0x00, 0x00, 0x00, 0x3c, // TTL (60 seconds)
            0x00, 0x04, // data length (4 bytes for IPv4)

            server_addr[0], server_addr[1], server_addr[2], server_addr[3] // IP address
        ]);

        // info!("Sending DNS response: {:?}", resp);
        
        resp
    }
}