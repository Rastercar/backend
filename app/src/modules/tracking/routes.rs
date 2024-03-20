use crate::{
    modules::{
        auth::{jwt, service::AuthService},
        common::responses::SimpleError,
    },
    server::controller::AppState,
};
use entity::vehicle_tracker;
use sea_orm::{entity::prelude::*, QuerySelect, QueryTrait};
use socketioxide::extract::{Data, SocketRef, State, TryData};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// The maximun amount of trackers a user can
/// listen to for realtime position updates
const TRACKER_SUBSCRIPTION_PER_USER_LIMIT: usize = 20;

/// Which users are listening to what trackers locations in real time
///
/// This is behind a RwLock because it will likely be read everytime a
/// position is recieved (that's a lot of reads), but written only whenever
/// a rastercar user changes what trackers he is interested in.
///
/// The `HashMap` key is the user ID and the value is a `Vec<i32>` of
/// tracker ID's because a user might listen up to only 20 trackers at
/// a given time so a vec is fine for performance and a HashSet would
/// probably be worse.
///
/// Since a entry is at most 21 bytes, 10k rastercar users would only
/// use 210 kilobytes of memory
///
/// TODO: think about memory leaks and TTL here
struct UserTrackersSubscription(RwLock<HashMap<i32, Vec<i32>>>);

/// The authenticated user connected to a socket
#[derive(Clone, Copy)]
struct SocketUser {
    /// The user organization ID, `None` if its a
    /// superuser and thus not bound to a single org
    pub org_id: Option<i32>,
}

// TODO: impl subscribe and unsubscribe methods
impl UserTrackersSubscription {}

#[derive(serde::Deserialize)]
pub struct AuthPayload {
    /// A short lived token for a rastercar API user
    pub token: String,
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
async fn on_change_trackers_to_listen(s: SocketRef, Data(mut tracker_ids): Data<Vec<i32>>) {
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

    tracker_ids = valid_tracker_ids;

    let rooms = tracker_ids
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();

    let _ = s.leave_all();
    let _ = s.join(rooms);
}

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

    let fetch_user_result = entity::user::Entity::find_by_id(user_id)
        .one(&state.db)
        .await;

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
