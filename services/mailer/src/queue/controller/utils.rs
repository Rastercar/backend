use lapin::{
    message::Delivery,
    options::{BasicAckOptions, BasicNackOptions},
    types::ShortString,
};

/// Gets the value from the `type` property, defaulting to `unknown`
pub fn get_delivery_type(delivery: &Delivery) -> String {
    delivery
        .properties
        .kind()
        .clone()
        .unwrap_or(ShortString::from("unknown"))
        .to_string()
}

pub async fn ack_delivery(delivery: &Delivery) -> Result<(), String> {
    delivery
        .ack(BasicAckOptions::default())
        .await
        .or(Err(create_ack_nack_error_string(delivery)))
}

pub async fn nack_delivery(delivery: &Delivery) -> Result<(), String> {
    delivery
        .nack(BasicNackOptions::default())
        .await
        .or(Err(create_ack_nack_error_string(delivery)))
}

pub fn create_ack_nack_error_string(delivery: &Delivery) -> String {
    format!(
        "error acking/nacking, delivery with tag: {} of type: {}",
        delivery.delivery_tag,
        get_delivery_type(delivery)
    )
}
