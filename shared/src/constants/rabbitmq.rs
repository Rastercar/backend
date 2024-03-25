/// RabbitMQ default exchange (yes, its a empty string)
pub static DEFAULT_EXCHANGE: &str = "";

/// RabbitMQ queue to be binded to the tracker events exchange
pub static TRACKER_EVENTS_QUEUE: &str = "tracker";

/// RabbitMQ queue to publish requests to the mailer service
pub static MAILER_QUEUE: &str = "mailer";

/// RabbitMQ exchange to listen to tracker events, such as positions and alerts
pub static TRACKER_EVENTS_EXCHANGE: &str = "tracker_events";

/// RPC operation to send a email
pub static OP_SEND_EMAIL: &str = "sendEmail";
