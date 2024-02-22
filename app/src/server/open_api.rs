use crate::modules::{auth, common, user, organization, vehicle, tracker, sim_card, access_level};
use crate::server::controller;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{ContactBuilder, InfoBuilder};
use utoipa::{openapi::OpenApiBuilder, Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_rapidoc::RapiDoc;
use axum::Router;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        shared::TrackerModel,

        entity::vehicle::Model,
        entity::sim_card::Model,
        entity::vehicle_tracker::Model,
        
        common::dto::PaginatedUser,
        common::dto::PaginatedSimCard,
        common::dto::PaginatedVehicle,
        common::dto::PaginatedVehicleTracker,

        common::dto::Token,
        common::dto::EmailAddress,
        common::dto::SingleImageDto,
        common::responses::SimpleError,
        
        user::dto::SimpleUserDto,
        user::dto::UpdateUserDto,
        user::dto::ChangePasswordDto,
        
        auth::dto::SignIn,
        auth::dto::UserDto,
        auth::dto::SessionDto,
        auth::dto::ResetPassword,
        auth::dto::SignInResponse,
        auth::dto::OrganizationDto,
        auth::dto::RegisterOrganization,

        vehicle::dto::CreateVehicleDto,
        vehicle::dto::UpdateVehicleDto,
        
        tracker::dto::Point,
        tracker::dto::UpdateTrackerDto,
        tracker::dto::CreateTrackerDto,
        tracker::dto::TrackerLocationDto,
        tracker::dto::SetTrackerVehicleDto,
        
        sim_card::dto::CreateSimCardDto,
        sim_card::dto::UpdateSimCardDto,
        sim_card::dto::SetSimCardTrackerDto,

        access_level::dto::AccessLevelDto,

        organization::dto::UpdateOrganizationDto,
    )),
    paths(
        controller::healthcheck,
        
        user::routes::me,
        user::routes::update_me,
        user::routes::list_users,
        user::routes::put_password,
        user::routes::get_user_sessions,
        user::routes::get_request_user_sessions,
        user::routes::put_profile_picture,
        user::routes::delete_profile_picture,
        user::routes::get_user_access_level,
        user::routes::request_user_email_address_confirmation,
        
        auth::routes::sign_up,
        auth::routes::sign_in,
        auth::routes::sign_out,
        auth::routes::delete_session,
        auth::routes::sign_out_session_by_id,
        auth::routes::request_recover_password_email,
        auth::routes::change_password_by_recovery_token,
        auth::routes::confirm_user_email_address_by_token,
        
        vehicle::routes::list_vehicles,
        vehicle::routes::vehicle_by_id,
        vehicle::routes::create_vehicle,
        vehicle::routes::update_vehicle,
        vehicle::routes::delete_vehicle,
        vehicle::routes::get_vehicle_tracker,
        vehicle::routes::update_vehicle_photo,
        vehicle::routes::delete_vehicle_photo,
        
        sim_card::routes::get_sim_card,
        sim_card::routes::list_sim_cards,
        sim_card::routes::delete_sim_card,
        sim_card::routes::create_sim_card,
        sim_card::routes::update_sim_card,
        sim_card::routes::set_sim_card_tracker,
        
        tracker::routes::get_tracker,
        tracker::routes::list_trackers,
        tracker::routes::create_tracker,
        tracker::routes::delete_tracker,
        tracker::routes::update_tracker,
        tracker::routes::set_tracker_vehicle,
        tracker::routes::get_tracker_location,
        tracker::routes::list_tracker_sim_cards,

        access_level::routes::list_access_level,
        access_level::routes::access_level_by_id,
        
        organization::routes::update_org,
        organization::routes::confirm_email_address_by_token,
        organization::routes::request_email_address_confirmation,
    ),
    modifiers(&SessionIdCookieSecurityScheme),
)]
struct ApiDoc;

/// session id on request cookie for user session authentication,
/// unfortunately this does not work on rapidoc or swagger UI for now, see:
///
/// https://github.com/swagger-api/swagger-js/issues/1163
struct SessionIdCookieSecurityScheme;

impl Modify for SessionIdCookieSecurityScheme {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // unfortunately as of writing this, the open api spec does not support 
            // scopes for apiKey authentication, such as cookies.
            components.add_security_scheme(
                "session_id",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
                    "sid",
                    "session identifier",
                ))),
            )
        }
    }
}

pub fn create_openapi_router() -> Router<controller::AppState> {
    let builder: OpenApiBuilder = ApiDoc::openapi().into();

    let info = InfoBuilder::new()
        .title("Rastercar API")
        .description(Some("Worlds best car tracking api."))
        .version("0.0.1")
        .contact(Some(
            ContactBuilder::new()
                .name(Some("Vitor Andrade Guidorizzi"))
                .email(Some("vitor.guidorizzi@hotmail.com"))
                .url(Some("https://github.com/VitAndrGuid"))
                .build(),
        ))
        .build();

    let api_doc = builder.info(info).build();

    Router::new()
        .merge(SwaggerUi::new("/swagger").url("/docs/openapi.json", api_doc))
        .merge(RapiDoc::new("/docs/openapi.json").path("/rapidoc"))
}
