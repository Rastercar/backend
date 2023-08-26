use lazy_static::lazy_static;
use serde::Deserialize;
use std::env;

fn def_http_port() -> u16 {
    3000
}

fn def_app_development() -> bool {
    false
}

fn def_db_url() -> String {
    String::from("postgres://raster_user:raster_pass@localhost/raster_dev")
}

lazy_static! {
    pub static ref ENV_DEVELOPMENT: bool = env::var("APP_DEVELOPMENT")
        .unwrap_or(def_app_development().to_string())
        .parse::<bool>()
        .unwrap_or(def_app_development());
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application is running in `development` mode
    #[serde(default = "def_app_development")]
    pub app_development: bool,

    #[serde(default = "def_http_port")]
    pub http_port: u16,

    #[serde(default = "def_db_url")]
    pub db_url: String,
}

impl AppConfig {
    /// loads the config from the environment variables
    ///
    /// # PANICS
    /// panics if the environment variables could not be loaded, such as when a string value
    /// cannot be parsed to the desired data type, eg:
    ///
    /// ENV_VAR_THAT_SHOULD_BE_BOOL=not_a_bool
    pub fn from_env() -> AppConfig {
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                if config.app_development {
                    println!("[CFG] {:#?}", config);
                }

                config
            }

            Err(error) => {
                panic!("[ENV] failed to load application config, {:#?}", error)
            }
        }
    }
}
