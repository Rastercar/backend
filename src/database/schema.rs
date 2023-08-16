// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "geometry"))]
    pub struct Geometry;
}

diesel::table! {
    access_level (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        description -> Varchar,
        is_fixed -> Bool,
        permissions -> Array<Nullable<Text>>,
        organization_id -> Nullable<Int4>,
    }
}

diesel::table! {
    master_access_level (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        description -> Varchar,
        is_fixed -> Bool,
        permissions -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    master_user (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        username -> Varchar,
        last_login -> Nullable<Timestamptz>,
        #[max_length = 255]
        email -> Varchar,
        email_verified -> Bool,
        #[max_length = 255]
        password -> Varchar,
        reset_password_token -> Nullable<Text>,
        confirm_email_token -> Nullable<Text>,
        access_level_id -> Nullable<Int4>,
        master_access_level_id -> Int4,
    }
}

diesel::table! {
    organization (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        name -> Varchar,
        deleted_at -> Nullable<Timestamptz>,
        blocked -> Bool,
        #[max_length = 255]
        billing_email -> Varchar,
        billing_email_verified -> Bool,
        owner_id -> Nullable<Int4>,
    }
}

diesel::table! {
    sim_card (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        phone_number -> Varchar,
        #[max_length = 255]
        ssn -> Varchar,
        #[max_length = 255]
        apn_address -> Varchar,
        #[max_length = 255]
        apn_user -> Varchar,
        #[max_length = 255]
        apn_password -> Varchar,
        #[max_length = 8]
        pin -> Nullable<Varchar>,
        #[max_length = 8]
        pin2 -> Nullable<Varchar>,
        #[max_length = 8]
        puk -> Nullable<Varchar>,
        #[max_length = 8]
        puk2 -> Nullable<Varchar>,
        organization_id -> Int4,
        tracker_id -> Nullable<Int4>,
    }
}

diesel::table! {
    unregistered_user (uuid) {
        #[max_length = 255]
        uuid -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        #[max_length = 255]
        username -> Nullable<Varchar>,
        #[max_length = 255]
        email -> Nullable<Varchar>,
        email_verified -> Bool,
        #[max_length = 255]
        oauth_provider -> Varchar,
        #[max_length = 255]
        oauth_profile_id -> Varchar,
    }
}

diesel::table! {
    user (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        username -> Varchar,
        last_login -> Nullable<Timestamptz>,
        #[max_length = 255]
        email -> Varchar,
        email_verified -> Bool,
        #[max_length = 255]
        password -> Varchar,
        reset_password_token -> Nullable<Text>,
        confirm_email_token -> Nullable<Text>,
        #[max_length = 255]
        profile_picture -> Nullable<Varchar>,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        #[max_length = 255]
        google_profile_id -> Nullable<Varchar>,
        auto_login_token -> Nullable<Text>,
        organization_id -> Int4,
        access_level_id -> Int4,
    }
}

diesel::table! {
    vehicle (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        plate -> Varchar,
        #[max_length = 255]
        photo -> Nullable<Varchar>,
        model_year -> Nullable<Int2>,
        fabrication_year -> Nullable<Int2>,
        #[max_length = 255]
        chassis_number -> Nullable<Varchar>,
        #[max_length = 255]
        brand -> Nullable<Varchar>,
        #[max_length = 255]
        model -> Nullable<Varchar>,
        #[max_length = 255]
        color -> Nullable<Varchar>,
        #[max_length = 255]
        fuel_type -> Nullable<Varchar>,
        fuel_consumption -> Nullable<Int4>,
        #[max_length = 255]
        additional_info -> Nullable<Varchar>,
        organization_id -> Int4,
    }
}

diesel::table! {
    vehicle_tracker (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        model -> Varchar,
        #[max_length = 255]
        imei -> Varchar,
        in_maintenance -> Bool,
        organization_id -> Int4,
        vehicle_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geometry;

    vehicle_tracker_last_location (tracker_id) {
        tracker_id -> Int4,
        time -> Timestamptz,
        point -> Geometry,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geometry;

    vehicle_tracker_location (time, tracker_id) {
        time -> Timestamptz,
        tracker_id -> Int4,
        point -> Geometry,
    }
}

diesel::joinable!(access_level -> organization (organization_id));
diesel::joinable!(master_user -> access_level (access_level_id));
diesel::joinable!(master_user -> master_access_level (master_access_level_id));
diesel::joinable!(sim_card -> organization (organization_id));
diesel::joinable!(sim_card -> vehicle_tracker (tracker_id));
diesel::joinable!(user -> access_level (access_level_id));
diesel::joinable!(vehicle -> organization (organization_id));
diesel::joinable!(vehicle_tracker -> organization (organization_id));
diesel::joinable!(vehicle_tracker -> vehicle (vehicle_id));
diesel::joinable!(vehicle_tracker_last_location -> vehicle_tracker (tracker_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_level,
    master_access_level,
    master_user,
    organization,
    sim_card,
    unregistered_user,
    user,
    vehicle,
    vehicle_tracker,
    vehicle_tracker_last_location,
    vehicle_tracker_location,
);
