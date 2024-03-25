use serde::Deserialize;

fn def_debug() -> bool {
    false
}

fn def_rmq_uri() -> String {
    "amqp://localhost:5672".to_string()
}

fn def_tracker_events_exchange() -> String {
    "tracker_events".to_string()
}

fn def_tracer_service_name() -> String {
    "tracker_decoder".to_string()
}

fn def_port_h02() -> usize {
    3003
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application should be run in debug mode and print additional info to stdout
    #[serde(default = "def_debug")]
    pub debug: bool,

    /// Rabbitmq uri
    #[serde(default = "def_rmq_uri")]
    pub rmq_uri: String,

    /// Name of the exchange to publish email events (positions, alarms, etc)
    #[serde(default = "def_tracker_events_exchange")]
    pub tracker_events_exchange: String,

    /// The service name to be used on the tracing spans
    #[serde(default = "def_tracer_service_name")]
    pub tracer_service_name: String,

    /// Default port to listen for trackers with the H02 protocol
    #[serde(default = "def_port_h02")]
    pub port_h02: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<AppConfig, envy::Error> {
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                if config.debug {
                    println!("[CFG] {:?}", config);
                }

                Ok(config)
            }

            Err(error) => Err(error),
        }
    }
}
