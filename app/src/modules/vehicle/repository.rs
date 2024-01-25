use super::dto::CreateVehicleDto;
use crate::database::error::DbError;
use crate::database::models_helpers::DbConn;
use crate::database::{models, schema};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub async fn create_vehicle(
    conn: &mut DbConn,
    dto: &CreateVehicleDto,
    org_id: i32,
) -> Result<models::Vehicle, DbError> {
    use schema::vehicle::dsl::*;

    Ok(diesel::insert_into(vehicle)
        .values((
            plate.eq(&dto.plate),
            brand.eq(&dto.brand),
            model.eq(&dto.model),
            color.eq(&dto.color),
            model_year.eq(&dto.model_year),
            chassis_number.eq(&dto.chassis_number),
            additional_info.eq(&dto.additional_info),
            organization_id.eq(org_id),
            fabrication_year.eq(&dto.fabrication_year),
        ))
        .get_result::<models::Vehicle>(conn)
        .await?)
}
