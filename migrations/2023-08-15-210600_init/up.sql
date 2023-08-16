-- Your SQL goes here
create table "master_access_level" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "name" varchar(255) not null,
    "description" varchar(255) not null,
    "is_fixed" boolean not null,
    "permissions" text [] not null default '{}'
);

create table "unregistered_user" (
    "uuid" varchar(255) not null,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) not null,
    "username" varchar(255) null,
    "email" varchar(255) null,
    "email_verified" boolean not null default false,
    "oauth_provider" varchar(255) not null,
    "oauth_profile_id" varchar(255) not null,
    constraint "unregistered_user_pkey" primary key ("uuid")
);

create table "organization" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "name" varchar(255) not null,
    "deleted_at" timestamptz(0) null,
    "blocked" boolean not null,
    "billing_email" varchar(255) not null,
    "billing_email_verified" boolean not null default false,
    "owner_id" int null
);

alter table
    "organization"
add
    constraint "organization_billing_email_unique" unique ("billing_email");

alter table
    "organization"
add
    constraint "organization_owner_id_unique" unique ("owner_id");

create table "user" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "username" varchar(255) not null,
    "last_login" timestamptz(0) null,
    "email" varchar(255) not null,
    "email_verified" boolean not null default false,
    "password" varchar(255) not null,
    "reset_password_token" text null,
    "confirm_email_token" text null,
    "profile_picture" varchar(255) null,
    "description" varchar(255) null,
    "google_profile_id" varchar(255) null,
    "auto_login_token" text null,
    "organization_id" int not null,
    "access_level_id" int not null
);

alter table
    "user"
add
    constraint "user_email_unique" unique ("email");

alter table
    "user"
add
    constraint "user_reset_password_token_unique" unique ("reset_password_token");

alter table
    "user"
add
    constraint "user_confirm_email_token_unique" unique ("confirm_email_token");

alter table
    "user"
add
    constraint "user_google_profile_id_unique" unique ("google_profile_id");

create table "access_level" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "name" varchar(255) not null,
    "description" varchar(255) not null,
    "is_fixed" boolean not null,
    "permissions" text [] not null default '{}',
    "organization_id" int null
);

create table "master_user" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "username" varchar(255) not null,
    "last_login" timestamptz(0) null,
    "email" varchar(255) not null,
    "email_verified" boolean not null default false,
    "password" varchar(255) not null,
    "reset_password_token" text null,
    "confirm_email_token" text null,
    "access_level_id" int null,
    "master_access_level_id" int not null
);

alter table
    "master_user"
add
    constraint "master_user_email_unique" unique ("email");

alter table
    "master_user"
add
    constraint "master_user_reset_password_token_unique" unique ("reset_password_token");

alter table
    "master_user"
add
    constraint "master_user_confirm_email_token_unique" unique ("confirm_email_token");

create table "vehicle" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "plate" varchar(255) not null,
    "photo" varchar(255) null,
    "model_year" smallint null,
    "fabrication_year" smallint null,
    "chassis_number" varchar(255) null,
    "brand" varchar(255) null,
    "model" varchar(255) null,
    "color" varchar(255) null,
    "fuel_type" varchar(255) null,
    "fuel_consumption" int null,
    "additional_info" varchar(255) null,
    "organization_id" int not null
);

alter table
    "vehicle"
add
    constraint "vehicle_plate_unique" unique ("plate");

create table "vehicle_tracker" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "model" varchar(255) not null,
    "imei" varchar(255) not null,
    "in_maintenance" boolean not null default false,
    "organization_id" int not null,
    "vehicle_id" int null
);

alter table
    "vehicle_tracker"
add
    constraint "vehicle_tracker_imei_unique" unique ("imei");

create table "sim_card" (
    "id" serial primary key,
    "created_at" timestamptz(0) not null,
    "updated_at" timestamptz(0) null,
    "phone_number" varchar(255) not null,
    "ssn" varchar(255) not null,
    "apn_address" varchar(255) not null,
    "apn_user" varchar(255) not null,
    "apn_password" varchar(255) not null,
    "pin" varchar(8) null,
    "pin2" varchar(8) null,
    "puk" varchar(8) null,
    "puk2" varchar(8) null,
    "organization_id" int not null,
    "tracker_id" int null
);

comment on column "sim_card"."phone_number" is 'Phone numbers are stored in the E164 international format';

alter table
    "sim_card"
add
    constraint "sim_card_phone_number_unique" unique ("phone_number");

alter table
    "sim_card"
add
    constraint "sim_card_ssn_unique" unique ("ssn");

create table "vehicle_tracker_last_location" (
    "tracker_id" int not null,
    "time" timestamptz(0) not null,
    "point" geometry not null,
    constraint "vehicle_tracker_last_location_pkey" primary key ("tracker_id")
);

alter table
    "vehicle_tracker_last_location"
add
    constraint "vehicle_tracker_last_location_tracker_id_unique" unique ("tracker_id");

create table "vehicle_tracker_location" (
    "time" timestamptz(0) not null,
    "tracker_id" int not null,
    "point" geometry not null,
    constraint "vehicle_tracker_location_pkey" primary key ("time", "tracker_id")
);

alter table
    "organization"
add
    constraint "organization_owner_id_foreign" foreign key ("owner_id") references "user" ("id") on update cascade on delete
set
    null;

alter table
    "user"
add
    constraint "user_organization_id_foreign" foreign key ("organization_id") references "organization" ("id") on update cascade;

alter table
    "user"
add
    constraint "user_access_level_id_foreign" foreign key ("access_level_id") references "access_level" ("id") on update cascade;

alter table
    "access_level"
add
    constraint "access_level_organization_id_foreign" foreign key ("organization_id") references "organization" ("id") on update cascade on delete
set
    null;

alter table
    "master_user"
add
    constraint "master_user_access_level_id_foreign" foreign key ("access_level_id") references "access_level" ("id") on update cascade on delete
set
    null;

alter table
    "master_user"
add
    constraint "master_user_master_access_level_id_foreign" foreign key ("master_access_level_id") references "master_access_level" ("id") on update cascade;

alter table
    "vehicle"
add
    constraint "vehicle_organization_id_foreign" foreign key ("organization_id") references "organization" ("id") on update cascade;

alter table
    "vehicle_tracker"
add
    constraint "vehicle_tracker_organization_id_foreign" foreign key ("organization_id") references "organization" ("id") on update cascade;

alter table
    "vehicle_tracker"
add
    constraint "vehicle_tracker_vehicle_id_foreign" foreign key ("vehicle_id") references "vehicle" ("id") on update cascade on delete
set
    null;

alter table
    "sim_card"
add
    constraint "sim_card_organization_id_foreign" foreign key ("organization_id") references "organization" ("id") on update cascade;

alter table
    "sim_card"
add
    constraint "sim_card_tracker_id_foreign" foreign key ("tracker_id") references "vehicle_tracker" ("id") on update cascade on delete
set
    null;

alter table
    "vehicle_tracker_last_location"
add
    constraint "vehicle_tracker_last_location_tracker_id_foreign" foreign key ("tracker_id") references "vehicle_tracker" ("id") on update cascade;