use serde::Deserialize;

fn def_http_port() -> u16 {
    3000
}

fn def_app_debug() -> bool {
    true
}

fn def_db_url() -> String {
    String::from("postgres://raster_user:raster_pass@localhost/raster_dev")
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application should be run in debug mode and print additional info to stdout
    #[serde(default = "def_app_debug")]
    pub app_debug: bool,

    #[serde(default = "def_http_port")]
    pub http_port: u16,

    #[serde(default = "def_db_url")]
    pub db_url: String,
}

impl AppConfig {
    /// loads the config from the environment variables
    ///
    /// # PANICS
    /// panics if the environment variables could not be loaded
    pub fn from_env() -> AppConfig {
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                if config.app_debug {
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
