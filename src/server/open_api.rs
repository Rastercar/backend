use crate::modules::{auth, common};
use crate::server::controller;
use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{ContactBuilder, InfoBuilder};
use utoipa::{openapi::OpenApiBuilder, Modify, OpenApi};
use utoipa_rapidoc::RapiDoc;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        common::responses::SimpleError,
        auth::dto::RegisterOrganization,
        auth::dto::SignInResponse,
        auth::dto::ForgotPassword,
        auth::dto::ResetPassword,
        auth::dto::UserDto,
        auth::dto::SignIn
    )),
    paths(
        controller::healthcheck,
        auth::routes::me,
        auth::routes::sign_up,
        auth::routes::sign_in,
        auth::routes::sign_out,
        auth::routes::recover_password,
        auth::routes::sign_out_session_by_id,
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
