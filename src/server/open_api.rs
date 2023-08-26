use crate::modules::{auth, common};
use crate::server::controller;
use axum::Router;
use utoipa::openapi::{ContactBuilder, InfoBuilder};
use utoipa::{openapi::OpenApiBuilder, OpenApi};
use utoipa_rapidoc::RapiDoc;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        common::responses::SimpleError,
        auth::dto::RegisterOrganization,
        auth::dto::SignInResponse,
        auth::dto::UserDto,
        auth::dto::SignIn
    )),
    paths(
        controller::healthcheck,
        auth::routes::sign_up,
        auth::routes::sign_in,
        auth::routes::sign_out,
        auth::routes::sign_out_session_by_id,
    )
)]
struct ApiDoc;

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
        .merge(SwaggerUi::new("/swagger").url("/docs/swagger.json", api_doc.clone()))
        .merge(RapiDoc::with_openapi("/docs/openapi.json", api_doc).path("/rapidoc"))
}
