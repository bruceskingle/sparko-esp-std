use std::io::Write;
use std::{backtrace::Backtrace, net::{IpAddr, UdpSocket}, sync::{Arc, Mutex}, thread};

use croner::Cron;
use esp_idf_hal::{gpio::PinDriver, ledc::LedcDriver};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, http::{Method, client::EspHttpConnection, server::EspHttpServer}, nvs::{EspDefaultNvsPartition, EspNvs}, timer::EspTaskTimerService};
use indexmap::IndexMap;
use log::{error, info};
use sparko_embedded_std::{SparkoEmbeddedStd, problem::ProblemManager, task::{Task, TaskManager, TaskManagerBuilder}};
use std::str::FromStr;
use esp_idf_svc::sntp::*;
use chrono::{Local, Utc};
use crate::esp_http_server::{HttpServer, TRequest};

#[cfg(feature = "board-xiao-esp32c6")]
use crate::led::MonoLedManager;
#[cfg(feature = "simple-led")]
use crate::led::SimpleLedManager;


use crate::{config::ConfigManagerBuilder, esp_http_server::{EspHttpServerManager, TMethod}, led::LedManager};
use crate::{Feature, config::{ConfigManager, SharedConfig}, core::{Core, MDNS_HOSTNAME}, led::RgbLedManager, wifi::WiFiManager};


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

        let config_manager_builder =  ConfigManager::builder(nvs_partition.clone(), problem_manager.clone(), ap_mode.clone())?;

        let mut builder = Self {
            nvs_partition,
            // failure_reason,
            problem_manager,
            features: Vec::new(),
            initializer: SparkoEsp32StdInitializer::new(),
            config_manager_builder,
            ap_mode,
        };

        builder.internal_add_feature(Box::new(Core::new()?), true)?;


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
        let config = self.config_manager_builder.add_feature(descriptor, internal)?;
        self.features.push(FeatureHolder {
            feature,
            config,
            name,
        });

        Ok(())
    }

    // pub fn with_handler<F>(
    //     mut self,
    //     uri: &str,
    //     method: Method,
    //     f: F,
    // ) -> Self
    // where
    //     F: for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> Self,
    // {
    //     let endpoint = Endpoint {
    //         uri: uri.to_string(),
    //         method,
    //     };

    //     let hndler = Box::new(f);
    //     self.handlers.insert(endpoint, Box::new(f));
    //     self

    // }

    pub fn build(mut self) -> anyhow::Result<SparkoEsp32StdRunner> {
        self.features.shrink_to_fit();
        
        let peripherals = Peripherals::take()?;


#[cfg(feature = "rgb-led")]
        let led_manager = RgbLedManager::new(true, 32, peripherals.ledc.timer0, 
            peripherals.ledc.channel0, peripherals.pins.gpio4, 
            peripherals.ledc.channel1, peripherals.pins.gpio16,
            peripherals.ledc.channel2, peripherals.pins.gpio17)?;
        
#[cfg(feature = "rgb-led")]
        led_manager.set_led_initializing()?;



#[cfg(feature = "board-xiao-esp32c6")]
        let led_manager = MonoLedManager::new(true,  peripherals.pins.gpio15)?;
        // let led_manager = SimpleLedManager::new(true, peripherals.pins.gpio15);

        led_manager.set_led_initializing()?;

        let sys_loop = EspSystemEventLoop::take()?;
        let timer_service = EspTaskTimerService::new()?;



        let wifi_manager = //wifi::wifi(peripherals.modem, sys_loop,Some(nvs_partition.clone()),timer_service)?;
            WiFiManager::new(peripherals.modem, sys_loop, self.nvs_partition.clone(), &self.problem_manager)?;

        // let led_red_pin = PinDriver::output(peripherals.pins.gpio4)?;
        // let led_green_pin = PinDriver::output(peripherals.pins.gpio16)?;
        // let led_blue_pin = PinDriver::output(peripherals.pins.gpio17)?;


        // let led_timer: esp_idf_hal::ledc::TIMER0<'_> = peripherals.ledc.timer0;
        // let led_timer_driver = esp_idf_hal::ledc::LedcTimerDriver::new(led_timer, &esp_idf_hal::ledc::config::TimerConfig::new().frequency(1000.Hz()))?;
    
        // let led_channel_red = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel0, &led_timer_driver, peripherals.pins.gpio4)?));
        // let led_channel_green = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel1, &led_timer_driver, peripherals.pins.gpio16)?));
        // let led_channel_blue = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel2, &led_timer_driver, peripherals.pins.gpio17)?));
        // let led = Arc::new(Mutex::new(led_pin));

        
        let mut bare_config_manager = //ConfigManager::new(nvs_partition, failure_reason, ap_mode.clone())?;
            self.config_manager_builder.build();

        // let mut features = Vec::new();

        

        // for feature in feature_list.into_iter() {
        //     let config = bare_config_manager.add_feature(&feature, false)?;
        //     features.push(FeatureHolder {
        //         feature: feature,
        //         config,
        //     });
        // }


        let mut server_manager = EspHttpServerManager::new()?;

        server_manager.init_common_pages()?;
        
        let config_manager = Arc::new(bare_config_manager);
        ConfigManager::create_pages(&config_manager, &mut server_manager)?;

        // This should be in the app

    let cloned_ap_mode = self.ap_mode.clone();
    server_manager.on("/", TMethod::Get, move |req: crate::esp_http_server::Request<'_, '_>| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            if cloned_ap_mode.lock().unwrap().clone() {
                let mut resp = req.into_response(
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
                // sntp: None,
            },
            initializer: self.initializer,
    })
    }
}

struct FeatureHolder {
    feature: Box<dyn Feature>,
    config: SharedConfig,
    name: String,
}


pub struct SparkoEsp32StdRunner {
    sparko_std: SparkoEsp32Std,
    initializer: SparkoEsp32StdInitializer,
}

impl SparkoEsp32StdRunner {
    pub fn start(mut self) -> anyhow::Result<()> {
        self.sparko_std.start(self.initializer)
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
    // task_manager: TaskManager,
    // sntp: Option<EspSntp<'static>>,
}

impl SparkoEmbeddedStd for SparkoEsp32Std {
    
}

impl SparkoEsp32Std {
    pub fn builder() -> anyhow::Result<SparkoEsp32StdBuilder> {
        SparkoEsp32StdBuilder::new()
    }

    

    fn start_client(&mut self, mut initializer: SparkoEsp32StdInitializer) -> anyhow::Result<()> {

        // start wifi

        let ip_address = self.wifi_manager.start_client(&self.config_manager)?;
        info!("Wifi started");

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

        self.config_manager.set_system_timezone()?;

        let local_time = Local::now();
        info!("Local time is: {}", local_time.format("%Y-%m-%d %H:%M:%S"));

        let hostname = self.config_manager.get_valid_core_config(MDNS_HOSTNAME)?;

        crate::mdns::start_mdns(&hostname, &ip_address)?;

        // Take features out to avoid borrowing issues
        let features = std::mem::take(&mut self.features);
        
        for feature_holder in &features {
            if feature_holder.config.enabled().is_enabled() {
                match feature_holder.feature.start( self, &mut initializer, &feature_holder.config) {
                    Ok(_) => info!("Started Feature {}", feature_holder.name),
                    Err(error) => error!("FAILED to start Feature {}: {}", feature_holder.name, error),
                }
            }
            else {
                info!("Feature {} is disabled", feature_holder.name)
            }
        }

        // Put features back
        self.features = features;
        let mut task_manager = initializer.task_manager_builder.build();

        self.led_manager.set_led_running()?;

        task_manager.run(self)
        
        
        // ; this is the core task now
        // loop {
        //         log::info!("Top of loop");

        //         let datetime = Utc::now();
        //         info!("Time synced: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

        
        //         let heap_free = unsafe { esp_get_free_heap_size() };
        //         let heap_min = unsafe { esp_get_minimum_free_heap_size() };
        //         log::info!("heap free={} min={}", heap_free, heap_min);
                
        //         // TODO: force a reset if we run low on heap

        //         std::thread::sleep(std::time::Duration::from_secs(10));
        //     }

        // Ok(())
    }
    

    // pub fn run(&mut self) -> anyhow::Result<()> {

    // }
    

    fn start(&mut self, initializer: SparkoEsp32StdInitializer) -> anyhow::Result<()> {
        log::info!("sparko_cyd: top of run");
        if self.config_manager.is_core_config_valid() {
            log::info!("Loaded config");

            if let Err(error) = self.start_client(initializer) {
                log::error!("Error starting client: {}", error);
                self.led_manager.set_led_error()?;
            }
            else {
                log::info!("Client mode started successfully");
                return Ok(());
            }



            // return Ok(());

            // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
            //         let mut response = req.into_ok_response()?;
            //         // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
            //         response.write(format!("Hello").as_bytes())?;
            //         response.flush()?;
            //         led.lock().unwrap().toggle()?;
            //         Ok(())
            //     })?;
                

            

            // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
            //     let mut response = req.into_ok_response()?;
            //     // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
            //     response.write(format!("External IP Address is: {}", handler_addr.lock().unwrap()).as_bytes())?;
            //     led.lock().unwrap().toggle()?;
            //     Ok(())
            // })?;

            
        }
        else {
            self.led_manager.set_led_admin()?;
            info!("Invalid config, starting AP mode");
        }

        // if let Some(reason) = self.config_manager.failure_reason.lock().unwrap().as_ref() {
        //         info!("APMODE Failure reason present, showing error message on config page: {}", reason);
        //     }
        //     else {
        //         info!("APMODE No failure reason, not showing error message on config page");
        //     }

        *self.ap_mode.lock().unwrap() = true;
        

        // self.server_manager.init_ap_pages()?;

        let server_addr = self.wifi_manager.start_access_point()?;

        thread::spawn(move || Self::captive_dns_server(server_addr));
        
        loop {
            log::info!("Top of AP loop");

            

            // let mut led = led.lock()?;
            // led.toggle()?;
            std::thread::sleep(std::time::Duration::from_secs(10));
        }

        fn system_halt<S: AsRef<str>>(s: S) {
            // TODO: Implement BSOD or similar system halt mechanism here
            println!("{}", s.as_ref());

            let bt = Backtrace::force_capture();
            println!("Stack trace:\n{bt}");

            std::process::exit(1);
        }
    }

    // pub fn main_loop(&mut self) -> anyhow::Result<()> {
    //     loop {
    //         let now = std::time::SystemTime::now();
    //         let datetime: DateTime<Local> = now.into();
    //         info!("Top of main loop time is : {}", datetime.format("%Y-%m-%d %H:%M:%S"));

    //         let wake = (now + Duration::hours(1))
    //             .with_minute(0).unwrap()
    //             .with_second(0).unwrap()
    //             .with_nanosecond(0).unwrap();
            
    //         std::thread::sleep(std::time::Duration::from_secs(10));
    //     }
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