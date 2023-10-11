use serde::Deserialize;
use std::sync::OnceLock;
use url::Url;

fn def_http_port() -> u16 {
    3000
}

fn def_is_development() -> bool {
    false
}

fn def_db_url() -> String {
    String::from("postgres://raster_user:raster_pass@localhost/raster_dev")
}

fn def_rmq_uri() -> String {
    String::from("amqp://localhost:5672")
}

fn def_frontend_url() -> Url {
    Url::parse("http://localhost:5173").expect("[CFG] invalid value for env var FRONTEND_URL")
}

fn def_jwt_secret() -> String {
    String::from("b6d870d5f22658902bdcd4799d47ea72ed8e3d091287313483df2545069aaee1")
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// if the application is running in `development` mode
    #[serde(default = "def_is_development")]
    pub is_development: bool,

    /// http port the api will listen for requests on
    #[serde(default = "def_http_port")]
    pub http_port: u16,

    /// postgres URL
    #[serde(default = "def_db_url")]
    pub db_url: String,

    /// rabbitmq uri
    #[serde(default = "def_rmq_uri")]
    pub rmq_uri: String,

    /// rastercar frontend url, eg: https://rastercar.homolog.com for homolog environments etc
    #[serde(default = "def_frontend_url")]
    pub frontend_url: Url,

    /// 256 bit secret used to generate Json Web Tokens
    #[serde(default = "def_jwt_secret")]
    pub jwt_secret: String,
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
            Ok(config) => config,
            Err(error) => {
                panic!("[CFG] failed to load application config, {:#?}", error)
            }
        }
    }
}

/// returns a global read only reference to the app configuration
pub fn app_config() -> &'static AppConfig {
    static INSTANCE: OnceLock<AppConfig> = OnceLock::new();
    INSTANCE.get_or_init(|| AppConfig::from_env())
}
