use std::str::FromStr;
use std::sync::Arc;

use croner::Cron;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use log::info;
use sparko_embedded_std::{config::{ConfigValue, TypedValue}, problem::ProblemManager, tz::{TIMEZONE_LEN, TimeZone}};




pub trait ConfigStore {
    fn erase_all(&self) -> anyhow::Result<()>;
    fn load(&self, name: &str, config_value: &mut ConfigValue);
    fn save(&self, name: &str, config_value: &mut ConfigValue, str_value: &str) -> anyhow::Result<()>;
    fn remove(&self, name: &str, config_value: &mut ConfigValue) -> anyhow::Result<()>;
}

pub struct EspConfigStore {
    pub nvs_namespace: EspNvs<NvsDefault>,
    pub problem_manager: Arc<ProblemManager>,
}

impl EspConfigStore {

    fn unwrap_and_log_esp<T>(&self, name: &str, config_value: &mut ConfigValue, result: Result<Option<T>, esp_idf_sys::EspError>) -> Option<T> {
                match result {
                    Ok(opt_str) => {
                        if opt_str.is_none() && config_value.required {
                            config_value.problem_id = self.problem_manager.set(config_value.problem_id, format!("Required value {} is missing", name));
                        }
                        else {
                            self.problem_manager.clear(config_value.problem_id);
                        }

                        opt_str
                    },
                    Err(error) => {
                        let err = format!("NVS error reading {}: {}", name, error);
                        log::error!("{}", &err);
                        config_value.problem_id = self.problem_manager.set(config_value.problem_id, err);
                        None
                    },
                }
    }

    fn unwrap_and_log_cron<T>(&self, name: &str, config_value: &mut ConfigValue, result: Result<T, croner::errors::CronError>) -> Option<T> {
                match result {
                    Ok(opt_str) => {

                        self.problem_manager.clear(config_value.problem_id);
                        Some(opt_str)
                    },
                    Err(error) => {
                        let err = format!("NVS error reading {}: {}", name, error);
                        log::error!("{}", &err);
                        config_value.problem_id = self.problem_manager.set(config_value.problem_id, err);
                        None
                    },
                }
    }
}

impl ConfigStore for EspConfigStore {
    fn erase_all(&self) -> anyhow::Result<()> {
        self.nvs_namespace.erase_all()?;
        Ok(())
    }
    fn load(&self, name: &str, config_value: &mut ConfigValue) {
        info!("Reading config value {} from NVS", name);
        let result = match &config_value.value {
            TypedValue::String(len_ref, _) => {
                let len = *len_ref; // copying to avoid borrow issues
                let mut buf = vec![0u8; (len as usize)+1];

                if let Some(str)= self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_str(name, buf.as_mut_slice())) {
                    TypedValue::String(len, Some(str.to_string()))
                } else {
                    TypedValue::String(len, None)
                }
            },
            TypedValue::Int32(_) => TypedValue::Int32(self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_i32(name))),
            TypedValue::Int64(_) => TypedValue::Int64(self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_i64(name))),
            TypedValue::Bool(_) => {
                let v = if let Some(value) = self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_u8(name)) {
                    value != 0
                } else {
                    false
                };
                TypedValue::Bool(v)
            },
            TypedValue::TimeZone(_) => {
                if let Some(str) = self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_str(name, &mut [0u8; TIMEZONE_LEN as usize])) {
                    if let Some(tz) = TimeZone::from_str(str) {
                        TypedValue::TimeZone(tz)
                    } else {
                        TypedValue::TimeZone(TimeZone::Utc)
                    }
                } else {
                    TypedValue::TimeZone(TimeZone::Utc)
                }
            },
            TypedValue::Cron(cron) => {
                let len = 64;
                let mut buf = vec![0u8; (len as usize)+1];

                if let Some(str)= self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_str(name, buf.as_mut_slice())) {
                    TypedValue::Cron(self.unwrap_and_log_cron(name, config_value, Cron::from_str(str)))
                    
                } else {
                    TypedValue::Cron(None)
                }
            },
        };
        info!("Finished reading config value {} from NVS: {:?}", name, result);
        config_value.value = result;
    }

    fn save(&self, name: &str, config_value: &mut ConfigValue, str_val: &str) -> anyhow::Result<()> {
        
                log::info!("Config value {} is {}", name, str_val);
                match config_value.value.from_str(str_val) {
                    Ok(new_value) => {
                        if config_value.value.is_none() || new_value != config_value.value {
                            log::info!("Config value {} changed from {:?} to {:?}", name, config_value.value, new_value);

                            config_value.value = new_value;
                            // Save to NVS
                            log::info!("Save to NVS Config value {} is {}", name, str_val);
                            match &config_value.value {
                                TypedValue::String(len, Some(val)) => {
                                    info!("Saving string value for {} to NVS: {}", name, val);
                                    match self.nvs_namespace.set_str(name, val) {
                                        Ok(_) => info!("Saved OK Config value {} is {}", name, str_val),
                                        Err(error) => log::error!("Failed to save Config value {} is {}: {}", name, str_val, error),
                                    }

                                    {
                                        let mut buf = vec![0u8; (*len as usize)+1];

                                        if let Some(str)= self.unwrap_and_log_esp(name, config_value, self.nvs_namespace.get_str(name, buf.as_mut_slice())) {
                                            info!("Read back of {} is {}", name, str)
                                        } else {
                                            info!("Read back of {} is None", name)
                                        }
                                    }
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
                                TypedValue::Cron(Some(val)) => {
                                    info!("Saving Cron value for {} to NVS: {}", name, val);
                                    self.nvs_namespace.set_str(name, val.as_str())?
                                },
                                _ => anyhow::bail!("Invalid config value for {}: {:?}", name, config_value.value),
                            };
                        }
                        else {
                            log::info!("Config value {} unchanged: {:?}", name, config_value.value);
                        }
                        Ok(())
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to parse config value for {}: {}", name, e);
                    }
                }
    }
    
    fn remove(&self, name: &str, config_value: &mut ConfigValue) -> anyhow::Result<()> {
        self.nvs_namespace.remove(name)?;
        config_value.value = config_value.value.to_none();
        if config_value.required {
            config_value.problem_id = self.problem_manager.set(config_value.problem_id, format!("Required value {} removed", name));
        }
        Ok(())
    }
}