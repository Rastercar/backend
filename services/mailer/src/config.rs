use std::sync::OnceLock;

use serde::Deserialize;

fn def_app_debug() -> bool {
    false
}

fn def_rmq_uri() -> String {
    String::from("amqp://localhost:5672")
}

fn def_rmq_queue() -> String {
    String::from("mailer")
}

fn def_rmq_consumer_tag() -> String {
    String::from("mailer_service_consumer")
}

fn def_tracer_service_name() -> String {
    String::from("mailer")
}

fn def_email_events_exchange() -> String {
    String::from("email_events")
}

fn def_aws_region() -> String {
    String::from("us-east-1")
}

fn def_app_default_email_sender() -> String {
    String::from("rastercar.tests.001@gmail.com")
}

fn def_aws_ses_tracking_config_set() -> String {
    String::from("email-events")
}

fn def_aws_ses_max_emails_per_second() -> u32 {
    1
}

fn def_http_port() -> u16 {
    3005
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application should be run in debug mode and print additional info to stdout
    #[serde(default = "def_app_debug")]
    pub app_debug: bool,

    /// The service name to be used on the tracing spans
    #[serde(default = "def_tracer_service_name")]
    pub tracer_service_name: String,

    /// Rabbitmq uri
    #[serde(default = "def_rmq_uri")]
    pub rmq_uri: String,

    /// Name of the rabbitmq queue this service will consume
    #[serde(default = "def_rmq_queue")]
    pub rmq_queue: String,

    /// Tag name for the rabbitmq consumer of the queue in rmq_queue
    #[serde(default = "def_rmq_consumer_tag")]
    pub rmq_consumer_tag: String,

    /// Name of the exchange to publish email events (clicks, opens, etc)
    #[serde(default = "def_email_events_exchange")]
    pub rmq_email_events_exchange: String,

    /// AWS region
    #[serde(default = "def_aws_region")]
    pub aws_region: String,

    /// Name of the SES configuration set to be used to track email events (clicks, opens, etc)
    #[serde(default = "def_aws_ses_tracking_config_set")]
    pub aws_ses_tracking_config_set: String,

    /// AWS ARN of the SNS subscription used to publish tracked email events to this service,
    /// important to validate the sender of email events, if None validation wont be applied
    pub aws_sns_tracking_subscription_arn: Option<String>,

    /// Maximum amount of sendEmail operations per second for the AWS account.
    /// defaults to 1, the value for sandbox accounts
    /// see: https://docs.aws.amazon.com/ses/latest/dg/manage-sending-quotas.html
    #[serde(default = "def_aws_ses_max_emails_per_second")]
    pub aws_ses_max_emails_per_second: u32,

    #[serde(default = "def_http_port")]
    pub http_port: u16,

    /// Email address to be used to send emails if the caller does not specify a address
    #[serde(default = "def_app_default_email_sender")]
    pub app_default_email_sender: String,
}

impl AppConfig {
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
    INSTANCE.get_or_init(AppConfig::from_env)
}
