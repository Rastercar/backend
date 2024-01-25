use super::dto::CreateVehicleDto;
use crate::database::error::DbError;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub async fn create_vehicle(
    conn: &DatabaseConnection,
    dto: &CreateVehicleDto,
    org_id: i32,
) -> Result<entity::vehicle::Model, DbError> {
    let vehicle = entity::vehicle::ActiveModel {
        plate: Set(dto.plate.clone()),
        brand: Set(Some(dto.brand.clone())),
        model: Set(Some(dto.model.clone())),
        color: Set(dto.color.clone()),
        model_year: Set(dto.model_year),
        chassis_number: Set(dto.chassis_number.clone()),
        additional_info: Set(dto.additional_info.clone()),
        organization_id: Set(org_id),
        fabrication_year: Set(dto.fabrication_year),
        ..Default::default()
    };

    Ok(vehicle.insert(conn).await?)
}
