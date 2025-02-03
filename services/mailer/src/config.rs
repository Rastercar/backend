use std::{env, sync::OnceLock};

use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application should be run in debug mode and print additional info to stdout
    pub app_debug: bool,

    /// The service name to be used on the tracing spans
    pub tracer_service_name: String,

    /// Rabbitmq uri
    pub rmq_uri: String,

    /// Name of the rabbitmq queue this service will consume
    pub rmq_queue: String,

    /// Tag name for the rabbitmq consumer of the queue in rmq_queue
    pub rmq_consumer_tag: String,

    /// Name of the exchange to publish email events (clicks, opens, etc)
    pub rmq_email_events_exchange: String,

    /// AWS region
    pub aws_region: String,

    /// Name of the SES configuration set to be used to track email events (clicks, opens, etc)
    pub aws_ses_tracking_config_set: String,

    /// AWS ARN of the SNS subscription used to publish tracked email events to this service,
    /// important to validate the sender of email events, if None validation wont be applied
    pub aws_sns_tracking_subscription_arn: Option<String>,

    /// Maximum amount of sendEmail operations per second for the AWS account.
    /// defaults to 1, the value for sandbox accounts
    /// see: https://docs.aws.amazon.com/ses/latest/dg/manage-sending-quotas.html
    pub aws_ses_max_emails_per_second: u32,

    /// HTTP port used to recieve SNS events
    pub http_port: u16,

    /// Email address to be used to send emails if the caller does not specify a address
    pub app_default_email_sender: String,

    /// opentelemetry exporter endpoint
    pub otel_exporter_otlp_endpoint: String,
}

impl AppConfig {
    pub fn from_env() -> AppConfig {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let base_path = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();

        let yaml_config_file = File::with_name(&format!("{base_path}/env/{run_mode}.yaml"))
            .format(config::FileFormat::Yaml)
            .required(false);

        Config::builder()
            .add_source(yaml_config_file)
            .add_source(Environment::default())
            .build()
            .unwrap_or_else(|error| panic!("[CFG] error loading config, {:#?}", error))
            .try_deserialize::<AppConfig>()
            .unwrap_or_else(|error| panic!("[CFG] error deserializing config, {:#?}", error))
    }
}

/// returns a global read only reference to the app configuration
pub fn app_config() -> &'static AppConfig {
    static INSTANCE: OnceLock<AppConfig> = OnceLock::new();
    INSTANCE.get_or_init(AppConfig::from_env)
}
