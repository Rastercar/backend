use crate::modules::user;
use axum::body::Bytes;
use axum_typed_multipart::{FieldData, TryFromMultipart};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
pub struct EmailAddress {
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct Token {
    #[validate(length(min = 5))]
    pub token: String,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    10
}

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    #[serde(default = "default_page")]
    #[validate(range(min = 1, max = 99999))]
    pub page: u64,

    #[serde(default = "default_page_size")]
    #[validate(range(min = 1, max = 100))]
    pub page_size: u64,
}

/// Pagination metadata of a executed query.
///
/// this struct also requires `T` on the records field to implement
/// `utoipa::ToSchema` since this struct is intended to be used as
/// a API response with openApi docs generation
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[aliases(
    PaginatedUser = PaginationResult<user::dto::SimpleUserDto>,
    PaginatedVehicle = PaginationResult<entity::vehicle::Model>,
    PaginatedSimCard = PaginationResult<entity::sim_card::Model>,
    PaginatedVehicleTracker = PaginationResult<entity::vehicle_tracker::Model>
)]
pub struct PaginationResult<T: for<'_s> ToSchema<'_s>> {
    /// 1 Indexed Page number
    ///
    /// used to determine the offset used in the query
    pub page: u64,

    /// Total pages available for the given query
    pub page_count: u64,

    /// Total items available for the given query
    pub item_count: u64,

    /// Amount of records per page
    pub page_size: u64,

    /// Records from the query
    pub records: Vec<T>,
}

/// DTO to send a image, should be extracted from `multipart/form-data`
/// requests containing a single field `image` field
#[derive(TryFromMultipart, ToSchema)]
pub struct SingleImageDto {
    #[schema(value_type = String, format = Binary)]
    pub image: FieldData<Bytes>,
}
