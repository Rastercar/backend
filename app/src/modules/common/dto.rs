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

/// Pagination metadata of a executed query
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[aliases(
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
