use super::dto::{AuthPayload, GetTrackersLastPositionsDto, PositionDto};
use crate::{
    modules::{
        auth::{self, jwt, service::AuthService},
        common::{
            extractors::{DbConnection, OrganizationId, ValidatedJson},
            responses::{internal_error_res, SimpleError},
        },
    },
    server::controller::AppState,
};
use axum::{routing::post, Json, Router};
use chrono::{DateTime, Utc};
use http::StatusCode;
use sea_orm::{entity::prelude::*, QuerySelect, QueryTrait};
use sea_query::{Cond, PostgresQueryBuilder, Query as SeaQuery};
use sea_query_binder::SqlxBinder;
use shared::entity::{user, vehicle_tracker, vehicle_tracker_last_location};
use socketioxide::extract::{Data, SocketRef, State, TryData};

/// The maximun amount of trackers a user can
/// listen to for realtime position updates
const TRACKER_SUBSCRIPTION_PER_USER_LIMIT: usize = 20;

/// The authenticated user connected to a socket
#[derive(Clone, Copy)]
struct SocketUser {
    /// The user organization ID, `None` if its a
    /// superuser and thus not bound to a single org
    pub org_id: Option<i32>,
}

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/positions/per-day", post(get_positions_per_day))
        .route("/last-positions", post(get_trackers_last_positions))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::middleware::require_user,
        ))
}

/// Gets the amount of positions recieved grouped by days
#[utoipa::path(
    post,
    tag = "tracking",
    path = "/tracking/positions/per-day",
    security(("session_id" = [])),
    // TODO:
    request_body = GetTrackersLastPositionsDto,
    responses(
        (
            status = OK,
            description = "the positions count per day",
            // TODO:
            body = Vec<PositionDto>,
            content_type = "application/json",
        ),
    ),
)]
pub async fn get_positions_per_day(
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
    ValidatedJson(dto): ValidatedJson<GetTrackersLastPositionsDto>,
) -> Result<Json<Vec<PositionDto>>, (StatusCode, SimpleError)> {
    // TODO:
    // SELECT DATE_TRUNC('day', time) AS day, count(*)
    // FROM vehicle_tracker_location
    // WHERE time >= '2023-02-01' AND time <= '2023-02-01'::timestamp + INTERVAL '7 days'
    // GROUP BY DATE_TRUNC('day', time);

    todo!();
}

/// Gets the most recent positions of a few trackers
#[utoipa::path(
    post,
    tag = "tracking",
    path = "/tracking/last-positions",
    security(("session_id" = [])),
    request_body = GetTrackersLastPositionsDto,
    responses(
        (
            status = OK,
            description = "the trackers positions",
            body = Vec<PositionDto>,
            content_type = "application/json",
        ),
    ),
)]
#[tracing::instrument(
    skip_all,
    fields(
        org_id = %org_id,
    )
)]
pub async fn get_trackers_last_positions(
    DbConnection(db): DbConnection,
    OrganizationId(org_id): OrganizationId,
    ValidatedJson(dto): ValidatedJson<GetTrackersLastPositionsDto>,
) -> Result<Json<Vec<PositionDto>>, (StatusCode, SimpleError)> {
    let valid_tracker_ids = match get_existing_tracker_ids(&db, Some(org_id), dto.ids.clone()).await
    {
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                SimpleError::from("failed to check tracker ids"),
            ));
        }
        Ok(ids) => ids,
    };

    let (q, args) = SeaQuery::select()
        .column(vehicle_tracker_last_location::Column::Time)
        .column(vehicle_tracker_last_location::Column::Point)
        .column(vehicle_tracker_last_location::Column::VehicleTrackerId)
        .from(vehicle_tracker_last_location::Entity)
        .cond_where(
            Cond::all().add(
                Expr::col(vehicle_tracker_last_location::Column::VehicleTrackerId)
                    .is_in(valid_tracker_ids),
            ),
        )
        .to_owned()
        .build_sqlx(PostgresQueryBuilder);

    let positions: Vec<PositionDto> = sqlx::query_as_with(&q, args)
        .fetch_all(db.get_postgres_connection_pool())
        .await
        .map_err(|_| internal_error_res())?
        .into_iter()
        .filter_map(
            |row: (
                DateTime<Utc>,
                geozero::wkb::Decode<geo_types::Geometry<f64>>,
                i32,
            )| {
                if let Some(geo_types::Geometry::Point(point)) = row.1.geometry {
                    let loc = PositionDto {
                        lat: point.y(),
                        lng: point.x(),
                        timestamp: row.0,
                        tracker_id: row.2,
                    };

                    return Some(loc);
                }

                None
            },
        )
        .collect();

    Ok(Json(positions))
}

/// Given a vec of tracker ids, return only those that
/// exists on the database
///
/// If `org_id` is `Some` trackers will also be filtered
/// by their organization_id
async fn get_existing_tracker_ids(
    db: &DatabaseConnection,
    maybe_org_id: Option<i32>,
    tracker_ids: Vec<i32>,
) -> Result<Vec<i32>, DbErr> {
    let cnt: Vec<i32> = vehicle_tracker::Entity::find()
        .select_only()
        .column(vehicle_tracker::Column::Id)
        .filter(vehicle_tracker::Column::Id.is_in(tracker_ids))
        .apply_if(maybe_org_id, |query, org_id| {
            query.filter(vehicle_tracker::Column::OrganizationId.eq(org_id))
        })
        .into_tuple()
        .all(db)
        .await?;

    Ok(cnt)
}

/// extracts a user ID from the JWT within a SocketIO payload
fn get_user_id_from_token(
    TryData(auth_payload): TryData<AuthPayload>,
    auth_service: &AuthService,
) -> anyhow::Result<i32> {
    let token = auth_payload?.token;
    let decoded_token = jwt::decode(&token)?;

    let user_id = auth_service.get_user_id_from_token_aud(decoded_token.claims.aud)?;

    Ok(user_id)
}

fn send_error(s: &SocketRef, msg: &str) {
    let _ = s.emit("error", SimpleError::from(msg));
}

/// Callback for the `change_trackers_to_listen` event.
///
/// Verifies the tracker ids informed by the event, and, for every tracker
/// that exists in the database and belong to the request user org starts
/// listening to positions for said tracker.
async fn on_change_trackers_to_listen(s: SocketRef, Data(tracker_ids): Data<Vec<i32>>) {
    if tracker_ids.len() > TRACKER_SUBSCRIPTION_PER_USER_LIMIT {
        let error_msg =
            format!("cannot listen to over {TRACKER_SUBSCRIPTION_PER_USER_LIMIT} trackers");

        send_error(&s, &error_msg);
        return;
    }

    let user = match s.extensions.get::<SocketUser>() {
        None => {
            send_error(&s, "internal server error getting user");
            return;
        }
        Some(u) => *u,
    };

    let db = match s.extensions.get::<DatabaseConnection>() {
        None => {
            send_error(&s, "internal server error getting DB conn");
            return;
        }
        Some(db) => db.clone(),
    };

    let valid_tracker_ids =
        match get_existing_tracker_ids(&db, user.org_id, tracker_ids.clone()).await {
            Err(_) => {
                let error_msg = "server error checking trackers to listen, list not updated";
                send_error(&s, error_msg);
                return;
            }
            Ok(ids) => ids,
        };

    let invalid_ids: Vec<&i32> = tracker_ids
        .iter()
        .filter(|id| !valid_tracker_ids.contains(id))
        .collect();

    if !invalid_ids.is_empty() {
        let ids = invalid_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        let error_msg = format!("cannot listen to not found trackers: {ids}");
        send_error(&s, &error_msg);
    }

    let rooms = valid_tracker_ids
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();

    let _ = s.leave_all();
    let _ = s.join(rooms);
}

/// callback for when a SocketIO connection is established
///
/// authenticates the user with the JWT with the connection payload
/// and stablishes the callbacks for client sent events
pub async fn on_connect(
    socket: SocketRef,
    State(state): State<AppState>,
    auth_payload: TryData<AuthPayload>,
) {
    let maybe_user_id = get_user_id_from_token(auth_payload, &state.auth_service);

    if maybe_user_id.is_err() {
        let _ = socket.disconnect();
        return;
    }

    let user_id = maybe_user_id.unwrap_or(0);

    let fetch_user_result = user::Entity::find_by_id(user_id).one(&state.db).await;

    if let Ok(Some(user)) = fetch_user_result {
        let socket_user = SocketUser {
            org_id: user.organization_id,
        };

        socket.extensions.insert(socket_user);
        socket.extensions.insert(state.db.clone());

        socket.on("change_trackers_to_listen", on_change_trackers_to_listen);

        return;
    }

    let _ = socket.disconnect();
}
