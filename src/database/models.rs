use chrono::offset::Utc;
use chrono::DateTime;
use diesel::{Identifiable, Queryable, Selectable};
use diesel_geometry::sql_types::*;
use ipnetwork::IpNetwork;

#[derive(Queryable, Debug, Identifiable, Selectable)]
#[diesel(table_name = crate::database::schema::access_level)]
#[diesel(belongs_to(Organization))]
pub struct AccessLevel {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub is_fixed: bool,

    /// permissions a user with this access level have
    ///
    /// unfortunately postgres arrays items are nullable so this is `Vec<Option<String>>` instead of `Vec<String>`
    pub permissions: Vec<Option<String>>,

    /// FK to the organization that created/owns this access level, if none
    /// this access level is to control admin users (users that do not belong to any organization)
    pub organization_id: Option<i32>,
}

#[derive(Queryable, Debug, Identifiable, Selectable, Clone)]
#[diesel(table_name = crate::database::schema::organization)]
#[diesel(belongs_to(Organization))]
pub struct Organization {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub blocked: bool,
    pub billing_email: String,
    pub billing_email_verified: bool,
    pub owner_id: Option<i32>,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(table_name = crate::database::schema::sim_card)]
#[diesel(belongs_to(Organization))]
pub struct SimCard {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub phone_number: String,
    pub ssn: String,
    pub apn_address: String,
    pub apn_user: String,
    pub apn_password: String,
    pub pin: Option<String>,
    pub pin2: Option<String>,
    pub puk: Option<String>,
    pub puk2: Option<String>,
    pub organization_id: i32,
    pub tracker_id: Option<i32>,
}

#[derive(Queryable, Debug, Identifiable, Selectable, Clone)]
#[diesel(table_name = crate::database::schema::user)]
#[diesel(belongs_to(Organization))]
pub struct User {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub password: String,

    /// JWT to be used to reset the user password
    ///
    /// note: this is stored in the database because this token needs to be one time
    /// use only and a simple solution is to clear this column after the token is used
    pub reset_password_token: Option<String>,

    /// JWT to be used to confirm the user email address
    ///
    /// note: this is stored in the database because this token needs to be one time
    /// use only and a simple solution is to clear this column after the token is used
    pub confirm_email_token: Option<String>,

    pub profile_picture: Option<String>,
    pub description: Option<String>,
    pub organization_id: Option<i32>,
    pub access_level_id: i32,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(table_name = crate::database::schema::vehicle)]
#[diesel(belongs_to(Organization))]
pub struct Vehicle {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub plate: String,
    pub photo: Option<String>,
    pub model_year: Option<i16>,
    pub fabrication_year: Option<i16>,
    pub chassis_number: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub fuel_type: Option<String>,
    pub fuel_consumption: Option<i32>,
    pub additional_info: Option<String>,
    pub organization_id: i32,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(table_name = crate::database::schema::vehicle_tracker)]
#[diesel(belongs_to(Organization))]
pub struct VehicleTracker {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub model: String,
    pub imei: String,
    pub in_maintenance: bool,
    pub organization_id: i32,
    pub vehicle_id: Option<i32>,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(tracker_id))]
#[diesel(table_name = crate::database::schema::vehicle_tracker_last_location)]
pub struct VehicleTrackerLastLocation {
    pub tracker_id: i32,
    pub time: DateTime<Utc>,
    pub point: Point,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(time, tracker_id))]
#[diesel(table_name = crate::database::schema::vehicle_tracker_location)]
pub struct VehicleTrackerLocation {
    pub time: DateTime<Utc>,
    pub tracker_id: i32,
    pub point: Point,
}

#[derive(Queryable, Debug, Identifiable, Selectable, Clone)]
#[diesel(primary_key(session_token))]
#[diesel(table_name = crate::database::schema::session)]
#[diesel(belongs_to(User))]
pub struct Session {
    /// A id that is safe to be exposed as its not used to authenticate a user.
    ///
    /// this can be used delete/list sessions
    pub public_id: i32,

    /// The u128 session id as vector of bytes to avoid leaking session ids
    pub session_token: Vec<u8>,

    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: String,
    pub ip: IpNetwork,
    pub user_id: i32,
}
