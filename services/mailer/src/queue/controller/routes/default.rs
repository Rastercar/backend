use crate::queue::controller::utils::nack_delivery;
use lapin::message::Delivery;

#[tracing::instrument(skip_all)]
pub async fn handle_delivery_without_corresponding_rpc(delivery: Delivery) -> Result<(), String> {
    nack_delivery(&delivery).await?;
    Err("handler does not exist".to_owned())
}
