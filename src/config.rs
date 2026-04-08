use esp_idf_svc::http::Method;
use esp_idf_svc::http::server::EspHttpConnection;

use indexmap::IndexMap;
use log::info;
use sparko_embedded_std::{config::{Config, EnabledState, TypedValue}, tz::{TIMEZONE_LEN, TimeZone}};
use url::form_urlencoded;
use crate::{core::{CORE_FEATURE_NAME, TIMEZONE}};
use std::{sync::{Arc, Mutex}};

use anyhow::anyhow;

// pub trait ConfigSerializable: std::fmt::Debug {
//     fn to_str(&self) -> &'static str;
//     fn from_str(&self, s: &str) -> Option<Box<Self>>;
//     fn iter_strs(&self) -> impl Iterator<Item = &'static str>;
// }







// impl ConfigValue {

    
    

//     fn read_from_nvs(&mut self, nvs: &EspNvs<NvsDefault>, name: &str) {
//         let nv = self.value.read_from_nvs(nvs, name);
//         self.value = nv;
//     }
// }



/// This is the descriptor for a feature which it uses to describe itself. 
#[derive(Debug)]
pub struct FeatureDescriptor {
    pub name: String,
    pub config: Config,
}

pub struct FeatureConfig {
    pub name: String,
    pub enabled: EnabledState,
    pub config: Config,
    nvs_namespace: EspNvs<NvsDefault>,
}

impl FeatureConfig {
    fn read_typed_value_from_nvs(typed_value: &TypedValue, nvs: &EspNvs<NvsDefault>, name: &str) -> TypedValue {
        info!("Reading config value {} from NVS", name);
        let result = match typed_value {
            TypedValue::String(len, _) => {
                let mut buf = vec![0u8; *len as usize];

                // let x = nvs.get_str(name, buf.as_mut_slice());
                // match x {
                //     Ok(str) => log::info!("Read string value for {} from NVS: {:?}", name, str),
                //     Err(e) => log::info!("No string value for {} in NVS: {:?}", name, e),
                // }

                if let Some(str)= nvs.get_str(name, buf.as_mut_slice()).ok().flatten() {
                    TypedValue::String(*len, Some(str.to_string()))
                } else {
                    TypedValue::String(*len, None)
                }
            },
            TypedValue::Int32(_) => TypedValue::Int32(nvs.get_i32(name).ok().flatten()),
            TypedValue::Int64(_) => TypedValue::Int64(nvs.get_i64(name).ok().flatten()),
            TypedValue::Bool(_) => {
                let v = if let Some(value) = nvs.get_u8(name).ok().flatten() {
                    value != 0
                } else {
                    false
                };
                TypedValue::Bool(v)
            },
            TypedValue::TimeZone(_) => {
                if let Some(str) = nvs.get_str(name, &mut [0u8; TIMEZONE_LEN as usize]).ok().flatten() {
                    if let Some(tz) = TimeZone::from_str(str) {
                        TypedValue::TimeZone(tz)
                    } else {
                        TypedValue::TimeZone(TimeZone::Utc)
                    }
                } else {
                    TypedValue::TimeZone(TimeZone::Utc)
                }
            },
        };
        info!("Finished reading config value {} from NVS: {:?}", name, result);
        result
    }

    pub fn from_feature(feature_descriptor: FeatureDescriptor, nvs_partition: EspNvsPartition<NvsDefault>, feature_namespace: &EspNvs<NvsDefault>, internal: bool) -> anyhow::Result<Self> {
        let enabled = if internal {
            EnabledState::Required
        }
        else {
        
            for reserved_name in RESERVED_FEATURE_NAMES.iter() {
                if feature_descriptor.name == *reserved_name {
                    return Err(anyhow::anyhow!("Feature name '{}' is reserved and cannot be used", feature_descriptor.name));
                }
            }

            let enabled = if let Some(value) = feature_namespace.get_u8(&feature_descriptor.name).ok().flatten() {
                info!("Read feature enabled value for {} from NVS: {}", feature_descriptor.name, value);
                value != 0
            } else {
                info!("Read feature enabled value for {} from NVS: None", feature_descriptor.name);
                false
            };

            EnabledState::from(enabled)
        };

        // info!("feature.enabled for {}: {}", feature_descriptor.name, enabled);
        Self::new(feature_descriptor.name, enabled, feature_descriptor.config, nvs_partition)
    }

    pub fn new(name: String, enabled: EnabledState, mut config: Config, nvs_partition: EspNvsPartition<NvsDefault>) -> anyhow::Result<Self> {

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

        info!("Loading feature {} config from NVS", &name);
        for (name, config_value) in config.map.iter_mut() {
            config_value.value = Self::read_typed_value_from_nvs(&config_value.value, &nvs_namespace, name);
        }
        info!("Finished loading config: {:?}", config);

        Ok(Self {
            name,
            enabled,
            config,
            nvs_namespace,
        })
    }

    pub fn is_valid(&self) -> bool {
        info!("Validating config for feature: {}", self.name);
        if self.enabled.is_enabled() {
            for (name, config_value) in &self.config.map {
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

    fn create_config_page(&self, resp: &mut esp_idf_svc::http::server::Response<&mut EspHttpConnection<'_>>) -> anyhow::Result<()> {
        info!("Creating config page for feature: {}", &self.name);
        let feature_name = &self.name;
        if let EnabledState::Required = self.enabled {
            // Required features are always enabled, so we just show the config page without a checkbox
        }
        else {
            info!("feature.enabled for {}: {}", &self.name, self.enabled.is_enabled());

            let name = format!("feature_{}", &self.name);
            let checked = if self.enabled.is_enabled() {
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

        for (name, config_value) in &self.config.map {
            let input_type_buf: String;
            let input_type = match config_value.value {
                TypedValue::String(len, _) => {
                    input_type_buf = format!("text\" maxlength=\"{}", len);
                    &input_type_buf
                },
                TypedValue::Int32(_) | TypedValue::Int64(_) => "number",
                TypedValue::Bool(_) => "checkbox",
                TypedValue::TimeZone(current) => {
                    info!("Config value {} is a TimeZone,", name);

                    resp.write(format!(r#"
                        <label for="{name}">{name}</label>
                        <select id="{name}" name="{name}">"#).as_bytes())?;
                    for tz in TimeZone::iter() {
                        let selected_attr = if *tz == current { " selected" } else { "" };
                        resp.write(format!(r#"<option value="{}"{}>{}</option>"#, tz.to_str(), selected_attr, tz.to_str()).as_bytes())?;
                    }
                    resp.write(format!(r#"</select>"#).as_bytes())?;
                    continue;
                },
            };
            let value = config_value.value.to_string();
            resp.write(format!(r#"
                        <label for="{name}">{name}</label>
                        <input id="{name}" name="{name}" type="{input_type}" autocomplete="off" value="{value}">
            "#).as_bytes())?;
        }

        Ok(())
    }

    pub fn handle_config_form(&mut self, form: &IndexMap<String, String>, feature_namespace: &EspNvs<NvsDefault>) -> anyhow::Result<()> {
        info!("Handling config form for feature: {}", self.name);
        if let EnabledState::Required = self.enabled {
            // Required features are always enabled, so we just show the config page without a checkbox
        }
        else {
            let name = format!("feature_{}", &self.name);
            let str_val = form.get(&name).map(|s| s.as_str()).unwrap_or("").trim();
            let enabled = str_val == "on";
            info!("Feature {} enabled value from form: {} -> enabled={}", &self.name, str_val, enabled);
            self.enabled = EnabledState::from(enabled);
                feature_namespace.set_u8(&self.name, if enabled { 1 } else { 0 })?;
        }

        for (name, config_value) in self.config.map.iter_mut() {
            info!("Processing config value: {}", name);
            let str_val = form.get(name).map(|s| s.as_str()).unwrap_or("").trim();
            if str_val.len() == 0 {
                if config_value.required {
                    log::error!("Missing required config value: {}", name);
                }
                else {
                    log::info!("Config value {} is None", name);
                    if ! config_value.value.is_none() {
                        log::info!("Setting optional config value {} to None", name);
                        config_value.value = config_value.value.to_none();
                        self.nvs_namespace.remove(name)?;
                    }
                }
            }
            else {
                log::info!("Config value {} is {}", name, str_val);
                match config_value.value.from_str(str_val) {
                    Ok(new_value) => {
                        if config_value.value.is_none() || new_value != config_value.value {
                            log::info!("Config value {} changed from {:?} to {:?}", name, config_value.value, new_value);

                            config_value.value = new_value;
                            // Save to NVS
                            log::info!("Save to NVS Config value {} is {}", name, str_val);
                            match &config_value.value {
                                TypedValue::String(_len, Some(val)) => {
                                    info!("Saving string value for {} to NVS: {}", name, val);
                                    self.nvs_namespace.set_str(name, val)?
                                },
                                TypedValue::Int32(Some(val)) => {
                                    info!("Saving int32 value for {} to NVS: {}", name, val);
                                    self.nvs_namespace.set_i32(name, *val)?
                                },
                                TypedValue::Int64(Some(val)) => {
                                    info!("Saving int64 value for {} to NVS: {}", name, val);
                                    self.nvs_namespace.set_i64(name, *val)?
                                },
                                TypedValue::Bool(val) => {
                                    info!("Saving bool value for {} to NVS: {}", name, val);
                                    self.nvs_namespace.set_u8(name, if *val { 1 } else { 0 })?
                                },
                                TypedValue::TimeZone(tz) => {
                                    info!("Saving TimeZone value for {} to NVS: {}", name, tz.to_str());
                                    self.nvs_namespace.set_str(name, tz.to_str())?
                                },
                                _ => anyhow::bail!("Invalid config value for {}: {:?}", name, config_value.value),
                            };
                        }
                        else {
                            log::info!("Config value {} unchanged: {:?}", name, config_value.value);
                        }
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to parse config value for {}: {}", name, e);
                    }
                }
            }
        }

        info!("Finished handling form config: {:?}", &self.config);

        info!("Iterating over NVS items for debugging:");
        let mut keys = self.nvs_namespace.keys(None).unwrap();

        loop {
            match keys.next_key() {
                Some((key, data_type)) => log::info!("NVS item: {} of type {:?}", key, data_type),
                None => break,
            }
        }

        Ok(())
    }
}


#[derive(Clone)]
pub struct SharedConfig(Arc<Mutex<FeatureConfig>>);

impl SharedConfig {

    pub fn new(feature_config: FeatureConfig) -> Self {
        SharedConfig(Arc::new(Mutex::new(feature_config)))
    }

    pub fn get_valid(&self, key: &str) -> anyhow::Result<String> {
        self.0.lock().unwrap().config.get_valid(key)
    }

    pub fn enabled(&self) -> EnabledState {
        self.0.lock().unwrap().enabled
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, FeatureConfig> {
        self.0.lock().unwrap()
    }
}

use esp_idf_svc::nvs::*;
use crate::http::HttpServerManager;

pub struct ConfigManagerBuilder {
    nvs_partition: EspNvsPartition<NvsDefault>,
    features: IndexMap<String, SharedConfig>,
    feature_namespace: EspNvs<NvsDefault>,
    failure_reason: Arc<Mutex<Option<String>>>,
    ap_mode: Arc<Mutex<bool>>,
}

impl ConfigManagerBuilder {
    fn new(
        nvs_partition: EspNvsPartition<NvsDefault>, 
        failure_reason: Arc<Mutex<Option<String>>>, 
        ap_mode: Arc<Mutex<bool>>) -> anyhow::Result<Self>
    {
        let features: IndexMap<String, SharedConfig> = IndexMap::new();
        let feature_namespace = EspNvs::new(nvs_partition.clone(), FEATURE_NAMESPACE_NAME, true)?;

        Ok(Self {
            nvs_partition,
            features,
            feature_namespace,
            failure_reason,
            ap_mode,
        })
    }

    pub fn add_feature(&mut self, descriptor: FeatureDescriptor, internal: bool) -> anyhow::Result<SharedConfig> {
        let feature_config = FeatureConfig::from_feature(descriptor, self.nvs_partition.clone(), &self.feature_namespace, internal)?;
        let feature_name = feature_config.name.clone();

        log::info!("Added feature: {}", &feature_name);

        let feature_mutex = SharedConfig::new(feature_config);
        self.features.insert(feature_name, feature_mutex.clone());

        log::info!("List ConfigManager:");
        for name in self.features.keys() {
            log::info!("Current feature in ConfigManager: {}", name);
        }
        log::info!("END List ConfigManager:");

        Ok(feature_mutex)
    }

    pub fn build(self) -> ConfigManager {
        ConfigManager {
            nvs_partition: self.nvs_partition,
            features: self.features,
            feature_namespace: self.feature_namespace,
            failure_reason: self.failure_reason,
            ap_mode: self.ap_mode,
        }
    }
}

pub struct ConfigManager {
    nvs_partition: EspNvsPartition<NvsDefault>,
    pub features: IndexMap<String, SharedConfig>,
    feature_namespace: EspNvs<NvsDefault>,
    pub failure_reason: Arc<Mutex<Option<String>>>,
    ap_mode: Arc<Mutex<bool>>,
}

impl ConfigManager {
    pub fn builder(
        nvs_partition: EspNvsPartition<NvsDefault>, 
        failure_reason: Arc<Mutex<Option<String>>>, 
        ap_mode: Arc<Mutex<bool>>)  -> anyhow::Result<ConfigManagerBuilder>
    {
        ConfigManagerBuilder::new(nvs_partition, failure_reason, ap_mode)
    }

    // pub fn new(nvs_partition: EspNvsPartition<NvsDefault>, 
    //     failure_reason: Arc<Mutex<Option<String>>>, 
    //     ap_mode: Arc<Mutex<bool>>) -> anyhow::Result<ConfigManager> {

        
    //     // // let core_descriptor = core.create_descriptor()?;

    //     // // let mut core_config = Config::new();

    //     // // core_config.insert(SSID.to_string(), ConfigValue { value: TypedValue::String(SSID_LEN, None), required: true })?;
    //     // // core_config.insert(WIFI_PASSWORD.to_string(), ConfigValue { value: TypedValue::String(PASSWORD_LEN, None), required: true })?;
    //     // // core_config.insert(MDNS_HOSTNAME.to_string(), ConfigValue { value: TypedValue::String(HOSTNAME_LEN, None), required: true })?;
    //     // // core_config.insert(TIMEZONE.to_string(), ConfigValue { value: TypedValue::TimeZone(TimeZone::Utc), required: true })?;

    //     // // let core_namespace = EspNvs::new(nvs_partition, "core", true)?;
    //     // let core_feature_config = FeatureConfig::new(
    //     //     core_descriptor.name.clone(),
    //     //     EnabledState::Required,
    //     //     core_descriptor.config,
    //     //     nvs_partition.clone(),
    //     // )?;
        
    //     // features.insert(core_descriptor.name, Mutex::new(core_feature_config));
        
    //     // for feature in p_features {
    //     //     let feature_config = FeatureConfig::from_feature(feature, nvs_partition.clone(), &feature_namespace)?;
    //     //     features.insert(feature_config.name.clone(), Mutex::new(feature_config));
    //     // }
        
        

    //     Ok(ConfigManager {
    //         nvs_partition,
    //         features,
    //         feature_namespace,
    //         failure_reason,
    //         ap_mode,
    //     })
    // }


    fn set_as_system_timezone(time_zone: &TimeZone) {
        let tz = std::ffi::CString::new(time_zone.to_posix_tz()).unwrap();
        unsafe {
            esp_idf_sys::setenv(b"TZ\0".as_ptr() as *const u8, tz.as_ptr(), 1);
            esp_idf_sys::tzset();
        }
        log::info!("System timezone set to {} ({})", time_zone.to_str(), time_zone.to_posix_tz());
    }

    pub fn set_system_timezone(&self) -> anyhow::Result<()> {
        let locked_config = self.features.get(CORE_FEATURE_NAME).unwrap().lock();
        let opt_config = locked_config.config.map.get(TIMEZONE);
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
        if let Some(value) = self.features.get(CORE_FEATURE_NAME).unwrap().lock().config.map.get(key) {
            Ok(value.value.to_string())
        }
        else {
            Err(anyhow!("Config value {} is missing", key))
        }
    }

    pub fn is_valid(&self) -> bool {
        for (_feature_name, feature_config_mutex) in &self.features {
            let feature_config = feature_config_mutex.lock();
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
        if let Some(core_feature_mutex) = self.features.get(CORE_FEATURE_NAME) {
            let core_feature = core_feature_mutex.lock();
            return core_feature.is_valid();
        }
        log::info!("Core feature not found");
        false
    }

    fn show_config_page(config_manager_clone: &Arc<ConfigManager>, req: esp_idf_svc::http::server::Request<&mut EspHttpConnection<'_>>) -> anyhow::Result<()> {


            let mut resp = req.into_ok_response()?;
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

            if let Some(reason) = config_manager_clone.failure_reason.lock().unwrap().as_ref() {
                info!("Failure reason present, showing error message on config page: {}", reason);
                resp.write(format!(r#"
                    <div style="background: #ffdddd; border: 1px solid #ff5c5c; padding: 10px; margin-bottom: 18px; border-radius: 8px;">
                        <strong>Error:</strong> {reason}
                    </div>
                "#).as_bytes())?;
            }
            else {
                info!("No failure reason, not showing error message on config page");
            }
            resp.write(r#"
                        <h1>ESP32 Setup</h1>
                        <form method="POST" action="/update_config">"#.as_bytes())?;
            for (_feature_name, feature_config_mutex) in &config_manager_clone.features {
                let feature_config = feature_config_mutex.lock();
                feature_config.create_config_page(&mut resp)?;
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

    pub fn create_pages(config_manager: &Arc<Self>, server_manager: &mut HttpServerManager<'_>) -> anyhow::Result<()> {
        let config_manager_clone = config_manager.clone();

        server_manager.fn_handler("/config", Method::Get, move |req| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            Self::show_config_page(&config_manager_clone, req)
        })?;

        let config_manager_clone = config_manager.clone();

        server_manager.fn_handler("/command", Method::Post, move |mut req| {
            info!("Received {:?} request for {}", req.method(), req.uri());
            

            let mut body = Vec::new();
            let mut buf = [0u8; 256];

            loop {
                let read = req.read(&mut buf)?;
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..read]);
            }

            let form = form_urlencoded::parse(&body)
                .into_owned()
                .collect::<IndexMap<String, String>>();

            let command =form.get("command");
            match command.map(|s| s.as_str()) {
                Some("restart") => {
                    info!("Restart command received, restarting...");
                    let mut resp = req.into_ok_response()?;
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
                        let mut resp = req.into_ok_response()?;
                        resp.write(b"<!doctype html><html><head><meta http-equiv=\"refresh\" content=\"5;url=/\" /><title>Factory reset failed</title></head><body><p>Failed to erase config.</p><script>setTimeout(()=>{window.location.href='/';},5000);</script></body></html>")?;
                    }
                    else {
                        let mut resp = req.into_ok_response()?;
                        resp.write(b"<!doctype html><html><head><meta http-equiv=\"refresh\" content=\"5;url=/\" /><title>Factory reset</title></head><body><p>Config erased. Device restarting, redirecting to root in 5 seconds...</p><script>setTimeout(()=>{window.location.href='/';},5000);</script></body></html>")?;
                    
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            unsafe { esp_idf_sys::esp_restart(); }
                        });
                    }
                },
                Some(cmd) => {
                    log::warn!("Unknown command received: {}", cmd);
                        let mut resp = req.into_ok_response()?;
                        resp.write(format!("Unknown command received: {}", cmd).as_bytes())?;
                },
                None => {
                    log::warn!("No command received in form");
                        let mut resp = req.into_ok_response()?;
                        resp.write(b"No command received in form")?;
                }
            }

            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();

        server_manager.fn_handler("/update_config", Method::Post, move |mut req| {

            // info!("Received request for /connect from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());
            

            let mut body = Vec::new();
            let mut buf = [0u8; 256];

            loop {
                let read = req.read(&mut buf)?;
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..read]);
            }

            let form = form_urlencoded::parse(&body)
                .into_owned()
                .collect::<IndexMap<String, String>>();

            config_manager_clone.handle_config_form(&form)?;

            Self::show_config_page(&config_manager_clone, req)

            // let mut resp = req.into_ok_response()?;
            // resp.write(b"Saved!. Rebooting...(NOT)")?;

            // // std::thread::spawn(|| {
            // //     std::thread::sleep(std::time::Duration::from_secs(2));
            // //     unsafe { esp_idf_sys::esp_restart(); }
            // // });

            // Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/generate_204", Method::Get, move |req| {

            let ok = config_manager_clone.is_online();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            
            if ok { 
                let mut resp = req.into_ok_response()?;        
                resp.write(b"<HTML><BODY>Success</BODY></HTML>")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/hotspot-detect.html", Method::Get, move |req| {

            let ok = config_manager_clone.is_online();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={} V2", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 3.2//EN\">
<HTML>
<HEAD>
	<TITLE>Success</TITLE>
</HEAD>
<BODY>
	Success
</BODY>
</HTML>")?;
            } else {let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/connecttest.txt", Method::Get, move |req| {

            let ok = config_manager_clone.is_online();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"Microsoft Connect Test")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"Not configured")?;
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn erase_config(&self) -> anyhow::Result<()> {
        info!("Erasing config");
        if let Some(core_feature_mutex) = self.features.get(CORE_FEATURE_NAME) {
            let core_feature = core_feature_mutex.lock();
            core_feature.nvs_namespace.erase_all()?;
        }
        Ok(())
    }

    pub fn handle_config_form(&self, form: &IndexMap<String, String>) -> anyhow::Result<()> {
        info!("Handling config form submission: {:?}", form);

        // Self::handle_config_form_feature(&mut self.nvs, form, None, &mut self.system_config.core_config)?;

        for (_feature_name, feature_config) in &self.features {
            feature_config.lock().handle_config_form(form, &self.feature_namespace)?;
        }

        // info!("Finished handling config form submission current config: {:?}", self.system_config);
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
