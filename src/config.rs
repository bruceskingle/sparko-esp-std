use std::io::Write;


use indexmap::IndexMap;
use log::info;
use sparko_embedded_std::{config::{Config, ConfigSpec, ConfigStoreFactory, EnabledState, TypedValue}, http_server::{HttpMethod, HttpServerManager}, problem::{ProblemId, ProblemManager}, tz::{TIMEZONE_LEN, TimeZone}};
use sparko_embedded_std::config::ConfigStore;
use crate::{core::{CORE_FEATURE_NAME, TIMEZONE}};