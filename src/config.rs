use std::io::Write;


use esp_idf_svc::http::{Method, server::{EspHttpConnection}};

use indexmap::IndexMap;
use log::info;
use sparko_embedded_std::{config::{Config, ConfigSpec, EnabledState, TypedValue}, http_server::{HttpMethod, HttpServerManager}, problem::{ProblemId, ProblemManager}, tz::{TIMEZONE_LEN, TimeZone}};
use sparko_embedded_std::config::ConfigStore;
use url::form_urlencoded;
use crate::{config_store::EspConfigStore, core::{CORE_FEATURE_NAME, TIMEZONE}, http::{EspHttpServerManager, WriteWrapper}};
use std::{sync::{Arc, Mutex}};

use esp_idf_svc::nvs::*;
// use crate::http::HttpServerManager;
use anyhow::anyhow;



/// This is the descriptor for a feature which it uses to describe itself. 
#[derive(Debug)]
pub struct FeatureDescriptor {
    pub name: String,
    pub config: ConfigSpec,
}

pub struct InnerFeatureConfig {
    pub enabled: EnabledState,
    pub config: ConfigSpec,
}

impl InnerFeatureConfig {
}

pub struct FeatureConfig {
    pub name: String,
    pub inner: Mutex<InnerFeatureConfig>,
    // nvs_namespace: EspNvs<NvsDefault>,
    // problem_manager: Arc<ProblemManager>,
    config_store: EspConfigStore,
}

impl FeatureConfig {
    // fn unwrap_and_log<T>(&self, name: &str, result: Result<Option<T>, esp_idf_sys::EspError>, problem_id: ProblemId) -> Option<T> {
    //             match result {
    //                 Ok(opt_str) => {
    //                     opt_str
    //                 },
    //                 Err(error) => {
    //                     let err = format!("NVS error reading {}: {}", name, error);
    //                     log::error!("{}", &err);
    //                     self.problem_manager.set(problem_id, err);
    //                     None
    //                 },
    //             }
    // }

    // fn read_typed_value_from_nvs(&self, nvs: &EspNvs<NvsDefault>, name: &str, config_value: &mut ConfigValue) -> anyhow::Result<TypedValue> {
    //     let typed_value: &TypedValue = &config_value.value;
    //     info!("Reading config value {} from NVS", name);
    //     let result = match typed_value {
    //         TypedValue::String(len, _) => {
    //             let mut buf = vec![0u8; (*len as usize)+1];

    //             if let Some(str)= self.unwrap_and_log(name, nvs.get_str(name, buf.as_mut_slice()), problem_id) {
    //                 TypedValue::String(*len, Some(str.to_string()))
    //             } else {
    //                 TypedValue::String(*len, None)
    //             }

    //             // // let x = nvs.get_str(name, buf.as_mut_slice());
    //             // // match x {
    //             // //     Ok(str) => log::info!("Read string value for {} from NVS: {:?}", name, str),
    //             // //     Err(e) => log::info!("No string value for {} in NVS: {:?}", name, e),
    //             // // }
    //             // let x: Result<Option<&str>, esp_idf_sys::EspError> = nvs.get_str(name, buf.as_mut_slice());
    //             // match nvs.get_str(name, buf.as_mut_slice()) {
    //             //     Ok(opt_str) => {
    //             //         if let Some(str)= opt_str {
    //             //             TypedValue::String(*len, Some(str.to_string()))
    //             //         } else {
    //             //             TypedValue::String(*len, None)
    //             //         }
    //             //     },
    //             //     Err(error) => {
    //             //         log::error!("NVS error reading {}: {}", name, error);
    //             //         anyhow::bail!("NVS error reading {}: {}", name, error);
    //             //     },
    //             // }
    //             // // if let Some(str)= nvs.get_str(name, buf.as_mut_slice()).ok().flatten() {
    //             // //     TypedValue::String(*len, Some(str.to_string()))
    //             // // } else {
    //             // //     TypedValue::String(*len, None)
    //             // // }
    //         },
    //         TypedValue::Int32(_) => TypedValue::Int32(self.unwrap_and_log(name, nvs.get_i32(name), problem_id)),
    //         TypedValue::Int64(_) => TypedValue::Int64(self.unwrap_and_log(name, nvs.get_i64(name), problem_id)),
    //         TypedValue::Bool(_) => {
    //             let v = if let Some(value) = self.unwrap_and_log(name, nvs.get_u8(name), problem_id) {
    //                 value != 0
    //             } else {
    //                 false
    //             };
    //             TypedValue::Bool(v)
    //         },
    //         TypedValue::TimeZone(_) => {
    //             if let Some(str) = self.unwrap_and_log(name, nvs.get_str(name, &mut [0u8; TIMEZONE_LEN as usize]), problem_id) {
    //                 if let Some(tz) = TimeZone::from_str(str) {
    //                     TypedValue::TimeZone(tz)
    //                 } else {
    //                     TypedValue::TimeZone(TimeZone::Utc)
    //                 }
    //             } else {
    //                 TypedValue::TimeZone(TimeZone::Utc)
    //             }
    //         },
    //     };
    //     info!("Finished reading config value {} from NVS: {:?}", name, result);
    //     Ok(result)
    // }



    pub fn to_config(&self) -> Config {
        let mut map = IndexMap::new();
        let inner = &self.inner.lock().unwrap();

        for (name, spec) in &inner.config.map {
            map.insert(name.clone(), spec.value.clone());
        }

        Config {
            enabled: inner.enabled,
            map,
        }
    }

    pub fn from_feature(feature_descriptor: FeatureDescriptor, nvs_partition: EspNvsPartition<NvsDefault>, feature_namespace: &EspNvs<NvsDefault>, internal: bool, problem_manager: &Arc<ProblemManager>) -> anyhow::Result<Self> {
        let enabled = if internal {
            EnabledState::Required
        }
        else {
        
            for reserved_name in RESERVED_FEATURE_NAMES.iter() {
                if feature_descriptor.name == *reserved_name {
                    return Err(anyhow::anyhow!("Feature name '{}' is reserved and cannot be used", feature_descriptor.name));
                }
            }

            let enabled = if let Some(value) = feature_namespace.get_u8(&feature_descriptor.name)? {
                info!("Read feature enabled value for {} from NVS: {}", feature_descriptor.name, value);
                value != 0
            } else {
                info!("Read feature enabled value for {} from NVS: None", feature_descriptor.name);
                false
            };

            EnabledState::from(enabled)
        };

        // info!("feature.enabled for {}: {}", feature_descriptor.name, enabled);
        Self::new(feature_descriptor.name, enabled, feature_descriptor.config, nvs_partition, problem_manager)
    }

    pub fn new(name: String, enabled: EnabledState, mut config: ConfigSpec, nvs_partition: EspNvsPartition<NvsDefault>, problem_manager: &Arc<ProblemManager>) -> anyhow::Result<Self> {

        let nvs_namespace = EspNvs::new(nvs_partition, &name, true)?;

        {
            info!("Iterating over feature {} NVS items for debugging:", &name);
            let mut keys = nvs_namespace.keys(None).unwrap();

            loop {
                match keys.next_key() {
                    Some((key, data_type)) => log::info!("NVS item: {} of type {:?}", key, data_type),
                    None => break,
                }
            }
        }

        let config_store = EspConfigStore{
            nvs_namespace,
            problem_manager: problem_manager.clone(),
        };

        info!("Loading feature {} config from NVS", &name);
        for (name, config_value) in config.map.iter_mut() {
            //config_value.value = 
            config_store.load(name, config_value);
        }
        info!("Finished loading config: {:?}", config);


        let feature_config = Self {
            name,
            inner: Mutex::new(InnerFeatureConfig { enabled, config }),
            // nvs_namespace,
            // problem_manager: problem_manager.clone(),
            config_store,
        };

        Ok(feature_config)
    }

    pub fn is_valid(&self) -> bool {
        info!("Validating config for feature: {}", self.name);
        let inner = &self.inner.lock().unwrap();
        if inner.enabled.is_enabled() {
            for (name, config_value) in &inner.config.map {
                if config_value.required && config_value.value.is_none() {
                    log::error!("Missing required config value: {} in feature {}", name, self.name);
                    return false;
                }
            }
        }
        else {
            info!("Config for feature {} is not enabled and therefore valid", self.name);
        }
        info!("Config for feature {} is valid", self.name);
        true
    }

    // fn create_config_page(&self, resp: &mut esp_idf_svc::http::server::Response<&mut EspHttpConnection<'_>>) -> anyhow::Result<()> {
    //     info!("Creating config page for feature: {}", &self.name);
    //     let feature_name = &self.name;
    //     if let EnabledState::Required = self.enabled {
    //         // Required features are always enabled, so we just show the config page without a checkbox
    //     }
    //     else {
    //         info!("feature.enabled for {}: {}", &self.name, self.enabled.is_enabled());

    //         let name = format!("feature_{}", &self.name);
    //         let checked = if self.enabled.is_enabled() {
    //             " checked"
    //         } else {
    //             ""
    //         };

    //         resp.write(format!(r#"
    //                     <label for="{name}">{name}</label>
    //                     <input id="{name}" name="{name}" type="checkbox"{checked}>
    //                     <h2>{feature_name}</h2>
    //         "#).as_bytes())?;
    //     }

    //     for (name, config_value) in &self.config.map {
    //         let input_type_buf: String;
    //         let input_type = match config_value.value {
    //             TypedValue::String(len, _) => {
    //                 input_type_buf = format!("text\" maxlength=\"{}", len);
    //                 &input_type_buf
    //             },
    //             TypedValue::Int32(_) | TypedValue::Int64(_) => "number",
    //             TypedValue::Bool(value) => {
    //                 let checked = if value {
    //                     " checked"
    //                 }
    //                 else {
    //                     ""
    //                 };

    //                 resp.write(format!(r#"
    //                             <label for="{name}">{name}</label>
    //                             <input id="{name}" name="{name}" type="checkbox" value="true" {checked}>
    //                 "#).as_bytes())?;
    //                 continue;
    //             },
    //             TypedValue::TimeZone(current) => {
    //                 info!("Config value {} is a TimeZone,", name);

    //                 resp.write(format!(r#"
    //                     <label for="{name}">{name}</label>
    //                     <select id="{name}" name="{name}">"#).as_bytes())?;
    //                 for tz in TimeZone::iter() {
    //                     let selected_attr = if *tz == current { " selected" } else { "" };
    //                     resp.write(format!(r#"<option value="{}"{}>{}</option>"#, tz.to_str(), selected_attr, tz.to_str()).as_bytes())?;
    //                 }
    //                 resp.write(format!(r#"</select>"#).as_bytes())?;
    //                 continue;
    //             },
    //         };
    //         let value = config_value.value.to_string();
    //         resp.write(format!(r#"
    //                     <label for="{name}">{name}</label>
    //                     <input id="{name}" name="{name}" type="{input_type}" autocomplete="off" value="{value}">
    //         "#).as_bytes())?;
    //     }

    //     Ok(())
    // }

    fn create_config_page(&self, 
        resp: &mut dyn std::io::Write
        //&mut esp_idf_svc::http::server::Response<&mut EspHttpConnection<'_>>
    ) -> anyhow::Result<()> {
        info!("Creating config page for feature: {}", &self.name);
        let feature_name = &self.name;
        let inner = &self.inner.lock().unwrap();
        if let EnabledState::Required = inner.enabled {
            // Required features are always enabled, so we just show the config page without a checkbox
        }
        else {
            info!("feature.enabled for {}: {}", &self.name, inner.enabled.is_enabled());

            let name = format!("feature_{}", &self.name);
            let checked = if inner.enabled.is_enabled() {
                " checked"
            } else {
                ""
            };

            resp.write(format!(r#"
                        <label for="{name}">{name}</label>
                        <input id="{name}" name="{name}" type="checkbox"{checked}>
                        <h2>{feature_name}</h2>
            "#).as_bytes())?;
        }

        for (name, config_value) in &inner.config.map {
            let value = config_value.value.to_string();
            let input_type_buf: String;
            let input_type = match &config_value.value {
                TypedValue::String(len, _) => {
                    input_type_buf = format!("text\" maxlength=\"{}", len);
                    &input_type_buf
                },
                TypedValue::Int32(_) | TypedValue::Int64(_) => "number",
                TypedValue::Bool(value) => {
                    let checked = if *value {
                        " checked"
                    }
                    else {
                        ""
                    };

                    resp.write(format!(r#"
                                <label for="{name}">{name}</label>
                                <input id="{name}" name="{name}" type="checkbox" value="true" {checked}>
                    "#).as_bytes())?;
                    continue;
                },
                TypedValue::TimeZone(current) => {
                    info!("Config value {} is a TimeZone,", name);

                    resp.write(format!(r#"
                                <!-- Timezone field {name}-->
                        <label for="{name}">{name}</label>
                        <select id="{name}" name="{name}">"#).as_bytes())?;
                    for tz in TimeZone::iter() {
                        let selected_attr = if *tz == *current { " selected" } else { "" };
                        resp.write(format!(r#"<option value="{}"{}>{}</option>"#, tz.to_str(), selected_attr, tz.to_str()).as_bytes())?;
                    }
                    resp.write(format!(r#"</select>"#).as_bytes())?;
                    continue;
                },
                TypedValue::Cron(opt_cron) => {
                    let desctiption = if let Some(cron) = opt_cron {
                        cron.describe()
                    }
                    else {
                        "None".to_string()
                    };

                    resp.write(format!(r#"
                                <!-- Cron field {name}-->
                                <label for="{name}">{name}</label>
                                <input id="{name}" name="{name}" type="text" value="{value}">
                                <input type="text" value="{desctiption}" disabled>
                    "#).as_bytes())?;
                    continue;
                },
            };
            resp.write(format!(r#"
                        <!-- Other field {name}-->
                        <label for="{name}">{name}</label>
                        <input id="{name}" name="{name}" type="{input_type}" autocomplete="off" value="{value}">
            "#).as_bytes())?;
        }

        Ok(())
    }

    pub fn handle_config_form(&self, form: &IndexMap<String, String>, feature_namespace: &EspNvs<NvsDefault>) -> anyhow::Result<()> {
        info!("Handling config form for feature: {}", self.name);
        let mut inner = self.inner.lock().unwrap();
        if let EnabledState::Required = inner.enabled {
            // Required features are always enabled, so we just show the config page without a checkbox
        }
        else {
            let name = format!("feature_{}", &self.name);
            let str_val = form.get(&name).map(|s| s.as_str()).unwrap_or("").trim();
            let enabled = str_val == "on";
            info!("Feature {} enabled value from form: {} -> enabled={}", &self.name, str_val, enabled);
            inner.enabled = EnabledState::from(enabled);
                feature_namespace.set_u8(&self.name, if enabled { 1 } else { 0 })?;
        }

        for (name, config_value) in inner.config.map.iter_mut() {
            info!("Processing config value: {}", name);
            let str_val = form.get(name).map(|s| s.as_str()).unwrap_or("").trim();
            if str_val.len() == 0 {
                log::info!("Config value {} is None", name);
                if ! config_value.value.is_none() {
                    log::info!("Setting optional config value {} to None", name);
                    self.config_store.remove(name, config_value)?;
                }
            }
            else {
                self.config_store.save(name, config_value, str_val)?;
            }
        }

        info!("Finished handling form config: {:?}", &inner.config);

        Ok(())
    }
}


// #[derive(Clone)]
// pub struct SharedConfig(Arc<Mutex<FeatureConfig>>);

// impl SharedConfig {

//     pub fn new(feature_config: FeatureConfig) -> Self {
//         SharedConfig(Arc::new(Mutex::new(feature_config)))
//     }

//     pub fn get_valid(&self, key: &str) -> anyhow::Result<String> {
//         self.0.lock().unwrap().config.get_valid(key)
//     }

//     pub fn enabled(&self) -> EnabledState {
//         self.0.lock().unwrap().enabled
//     }

//     pub fn lock(&self) -> std::sync::MutexGuard<'_, FeatureConfig> {
//         self.0.lock().unwrap()
//     }
// }


pub struct ConfigManagerBuilder {
    nvs_partition: EspNvsPartition<NvsDefault>,
    features: IndexMap<String, FeatureConfig>,
    feature_namespace: EspNvs<NvsDefault>,
    // failure_reason: Arc<Mutex<Option<String>>>,
    problem_manager: Arc<ProblemManager>,
    ap_mode: Arc<Mutex<bool>>,
}

impl ConfigManagerBuilder {
    fn new(
        nvs_partition: EspNvsPartition<NvsDefault>, 
        // failure_reason: Arc<Mutex<Option<String>>>, 
        problem_manager: Arc<ProblemManager>,
        ap_mode: Arc<Mutex<bool>>) -> anyhow::Result<Self>
    {
        let features: IndexMap<String, FeatureConfig> = IndexMap::new();
        let feature_namespace = EspNvs::new(nvs_partition.clone(), FEATURE_NAMESPACE_NAME, true)?;

        Ok(Self {
            nvs_partition,
            features,
            feature_namespace,
            // failure_reason,
            problem_manager,
            ap_mode,
        })
    }

    pub fn add_feature(&mut self, descriptor: FeatureDescriptor, internal: bool) -> anyhow::Result<Config> {
        log::info!("About to create config for feature: {}", &descriptor.name);
        let feature_config = FeatureConfig::from_feature(descriptor, self.nvs_partition.clone(), &self.feature_namespace, internal, &self.problem_manager)?;
        let feature_name = feature_config.name.clone();
        let config = feature_config.to_config();
        log::info!("Added feature: {}", &feature_name);

        self.features.insert(feature_name, feature_config);

        log::info!("List ConfigManager:");
        for name in self.features.keys() {
            log::info!("Current feature in ConfigManager: {}", name);
        }
        log::info!("END List ConfigManager:");

        Ok(config)
    }

    pub fn build(self) -> ConfigManager {
        ConfigManager {
            nvs_partition: self.nvs_partition,
            features: self.features,
            feature_namespace: self.feature_namespace,
            // failure_reason: self.failure_reason,
            problem_manager: self.problem_manager,
            ap_mode: self.ap_mode,
        }
    }
}

pub struct ConfigManager {
    nvs_partition: EspNvsPartition<NvsDefault>,
    pub features: IndexMap<String, FeatureConfig>,
    feature_namespace: EspNvs<NvsDefault>,
    // pub failure_reason: Arc<Mutex<Option<String>>>,
    problem_manager: Arc<ProblemManager>,
    ap_mode: Arc<Mutex<bool>>,
}

impl ConfigManager {
    pub fn builder(
        nvs_partition: EspNvsPartition<NvsDefault>, 
        // failure_reason: Arc<Mutex<Option<String>>>, 
        problem_manager: Arc<ProblemManager>,
        ap_mode: Arc<Mutex<bool>>)  -> anyhow::Result<ConfigManagerBuilder>
    {
        ConfigManagerBuilder::new(nvs_partition, problem_manager, ap_mode)
    }


    fn set_as_system_timezone(time_zone: &TimeZone) {
        let tz = std::ffi::CString::new(time_zone.to_posix_tz()).unwrap();
        unsafe {
            esp_idf_sys::setenv(b"TZ\0".as_ptr() as *const u8, tz.as_ptr(), 1);
            esp_idf_sys::tzset();
        }
        log::info!("System timezone set to {} ({})", time_zone.to_str(), time_zone.to_posix_tz());
    }

    pub fn set_system_timezone(&self) -> anyhow::Result<()> {
        let inner = self.features.get(CORE_FEATURE_NAME).unwrap().inner.lock().unwrap();
        let opt_config = &inner.config.map.get(TIMEZONE);
        if let Some(config) = opt_config {
            if let TypedValue::TimeZone(tz) = config.value {
                Self::set_as_system_timezone(&tz);
            }
            else {
                anyhow::bail!("Timezone config value has wrong type");
            }
        }
        else {
            Self::set_as_system_timezone(&TimeZone::Utc);
        }
        Ok(())
    }

    pub fn get_valid_core_config(&self, key: &str) -> anyhow::Result<String> {
        let inner = self.features.get(CORE_FEATURE_NAME).unwrap().inner.lock().unwrap();
        if let Some(value) = inner.config.map.get(key) {
            Ok(value.value.to_string())
        }
        else {
            Err(anyhow!("Config value {} is missing", key))
        }
    }

    pub fn is_valid(&self) -> bool {
        for (_feature_name, feature_config) in &self.features {
            if ! feature_config.is_valid() {
                return false;
            }
        }
        info!("ConfigManager is valid");
        true
    }

    pub fn is_online(&self) -> bool {
        let ap_mode = *self.ap_mode.lock().unwrap();
        info!("is_ap_mode: {}", ap_mode);
        !ap_mode
    }

    pub fn is_core_config_valid(&self) -> bool {
        log::info!("List ConfigManager:");
        for name in self.features.keys() {
            log::info!("Current feature in ConfigManager: {}", name);
        }
        log::info!("END List ConfigManager:");
        if let Some(core_feature) = self.features.get(CORE_FEATURE_NAME) {
            return core_feature.is_valid();
        }
        log::info!("Core feature not found");
        false
    }

    fn show_config_page(config_manager_clone: &Arc<ConfigManager>, 
        resp: &mut dyn Write
        //esp_idf_svc::http::server::Request<&mut EspHttpConnection<'_>>
        ) -> anyhow::Result<()> {


            // let mut resp = req.into_ok_response()?;
            resp.write(r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <title>ESP32 Setup</title>
                    <link rel="stylesheet" href="/main.css">
                </head>
                <body>
                    <div class="page">"#.as_bytes())?;
            
            

            if ! config_manager_clone.problem_manager.is_empty() {
                info!("Failure reason present, showing error message on config page");
                resp.write(format!(r#"
                    <div style="background: #ffdddd; border: 1px solid #ff5c5c; padding: 10px; margin-bottom: 18px; border-radius: 8px;">
                        <strong>Error:</strong> <ul>
                "#).as_bytes())?;

                for reason in &*config_manager_clone.problem_manager {
                    resp.write("<li>".as_bytes())?;
                    resp.write(reason.as_bytes())?;
                    resp.write("</li>\n".as_bytes())?;
                }

                resp.write(format!(r#"
                    </ul>
                    </div>
                "#).as_bytes())?;
            }
            else {
                info!("No failure reason, not showing error message on config page");
            }
            resp.write(r#"
                        <h1>ESP32 Setup</h1>
                        <form method="POST" action="/update_config">"#.as_bytes())?;
            for (_feature_name, feature_config) in &config_manager_clone.features {
                feature_config.create_config_page(resp)?;
            }

            
            resp.write(format!(r#"<button type="submit">Save</button>
                        </form>
                        <form method="POST" action="/command">
                        <label for="command">Command</label>
                            <select name="command" id="command">
                                <option value="restart">Restart</option>
                                <option value="factory_reset">Factory Reset</option>
                            </select>
                            <button type="submit">Execute</button>
                        </form>
                    </div>
                </body>
                </html>
                "#).as_bytes())?;
            Ok(())
    }

    pub fn create_pages(config_manager: &Arc<ConfigManager>, server_manager: &mut dyn HttpServerManager) -> anyhow::Result<()> {
        let config_manager_clone = config_manager.clone();

        server_manager.handle("/config", HttpMethod::Get, Box::new(move |resp| {
            Self::show_config_page(&config_manager_clone, resp)
        }))?;

        let config_manager_clone = config_manager.clone();

        server_manager.handle_post_form("/command", Box::new(move |mut resp, form| {
            let command =form.get("command");
            match command.map(|s| s.as_str()) {
                Some("restart") => {
                    info!("Restart command received, restarting...");
                    resp.write(b"<!doctype html><html><head><meta http-equiv=\"refresh\" content=\"5;url=/\" /><title>Restarting</title></head><body><p>Device restarting, redirecting to root in 5 seconds...</p><script>setTimeout(()=>{window.location.href='/';},5000);</script></body></html>")?;

                    std::thread::spawn(|| {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        unsafe { esp_idf_sys::esp_restart(); }
                    });
                },
                Some("factory_reset") => {
                    info!("Factory reset command received, erasing config and restarting...");
                    if let Err(e) = config_manager_clone.erase_config() {
                        log::error!("Failed to erase config: {}", e);
                        resp.write(b"<!doctype html><html><head><meta http-equiv=\"refresh\" content=\"5;url=/\" /><title>Factory reset failed</title></head><body><p>Failed to erase config.</p><script>setTimeout(()=>{window.location.href='/';},5000);</script></body></html>")?;
                    }
                    else {
                        resp.write(b"<!doctype html><html><head><meta http-equiv=\"refresh\" content=\"5;url=/\" /><title>Factory reset</title></head><body><p>Config erased. Device restarting, redirecting to root in 5 seconds...</p><script>setTimeout(()=>{window.location.href='/';},5000);</script></body></html>")?;
                    
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            unsafe { esp_idf_sys::esp_restart(); }
                        });
                    }
                },
                Some(cmd) => {
                    log::warn!("Unknown command received: {}", cmd);
                        resp.write(format!("Unknown command received: {}", cmd).as_bytes())?;
                },
                None => {
                    log::warn!("No command received in form");
                        resp.write(b"No command received in form")?;
                }
            }

            Ok(())
        }))?;

        let config_manager_clone = config_manager.clone();

        server_manager.handle_post_form("/update_config", Box::new(move | resp, form | {
            config_manager_clone.handle_config_form(&form)?;
            Self::show_config_page(&config_manager_clone, resp)
        }))?;

        Ok(())
    }

    pub fn erase_config(&self) -> anyhow::Result<()> {
        info!("Erasing config");
        if let Some(core_feature) = self.features.get(CORE_FEATURE_NAME) {
            core_feature.config_store.erase_all()?;
        }
        Ok(())
    }

    pub fn handle_config_form(&self, form: &IndexMap<String, String>) -> anyhow::Result<()> {
        info!("Handling config form submission: {:?}", form);
        for (_feature_name, feature_config) in &self.features {
            feature_config.handle_config_form(form, &self.feature_namespace)?;
        }
        Ok(())
    }
}




const FEATURE_NAMESPACE_NAME: &str = "feature";
const RESERVED_FEATURE_NAMES: [&str; 6] = [
    CORE_FEATURE_NAME,
    FEATURE_NAMESPACE_NAME,
    "wifi",
    "phy",
    "bt_config",
    "nvs.net80211",
];
