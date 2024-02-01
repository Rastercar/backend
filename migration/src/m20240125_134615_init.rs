use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let statement = r#"
        create table "organization" (
            "id" serial primary key,
            "created_at" timestamptz(0) not null default now(),
            "name" varchar(255) not null,
            "blocked" boolean not null,
            "billing_email" varchar(255) not null,
            "billing_email_verified" boolean not null default false,
            "confirm_billing_email_token" text null,
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
            "created_at" timestamptz(0) not null default now(),
            "username" varchar(255) not null,
            "email" varchar(255) not null,
            "email_verified" boolean not null default false,
            "password" varchar(255) not null,
            "reset_password_token" text null,
            "confirm_email_token" text null,
            "profile_picture" varchar(255) null,
            "description" text null,
            "organization_id" int,
            "access_level_id" int not null
        );
        
        alter table
            "user"
        add
            constraint "user_email_unique" unique ("email");
        
        alter table
            "user"
        add
            constraint "user_username_unique" unique ("username");
        
        alter table
            "user"
        add
            constraint "user_reset_password_token_unique" unique ("reset_password_token");
        
        alter table
            "user"
        add
            constraint "user_confirm_email_token_unique" unique ("confirm_email_token");
        
        create table "access_level" (
            "id" serial primary key,
            "created_at" timestamptz(0) not null default now(),
            "name" varchar(255) not null,
            "description" text not null,
            "is_fixed" boolean not null,
            "permissions" text [] not null default '{}',
            "organization_id" int null
        );
        
        create table "vehicle" (
            "id" serial primary key,
            "created_at" timestamptz(0) not null default now(),
            "plate" varchar(255) not null,
            "photo" varchar(255) null,
            "model_year" smallint null,
            "fabrication_year" smallint null,
            "chassis_number" varchar(255) null,
            "brand" varchar(255) null,
            "model" varchar(255) null,
            "color" varchar(255) null,
            "additional_info" varchar(255) null,
            "organization_id" int not null
        );
        
        alter table
            "vehicle"
        add
            constraint "vehicle_plate_unique" unique ("plate", "organization_id");
        
        create table "vehicle_tracker" (
            "id" serial primary key,
            "created_at" timestamptz(0) not null default now(),
            "model" varchar(255) not null,
            "imei" varchar(255) not null,
            "organization_id" int not null,
            "vehicle_id" int null
        );
        
        alter table
            "vehicle_tracker"
        add
            constraint "vehicle_tracker_imei_unique" unique ("imei", "organization_id");

        alter table
            "vehicle_tracker"
        add
            constraint "vehicle_tracker_vehicle_id_unique" unique ("vehicle_id");
        
        create table "sim_card" (
            "id" serial primary key,
            "created_at" timestamptz(0) not null default now(),
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
            constraint "sim_card_phone_number_unique" unique ("phone_number", "organization_id");
        
        alter table
            "sim_card"
        add
            constraint "sim_card_ssn_unique" unique ("ssn", "organization_id");
        
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
        
        create table "session" (
            "public_id" serial unique,
            "session_token" BYTEA PRIMARY KEY,
            "created_at" timestamptz(0) not null default now(),
            "expires_at" timestamptz(0) not null,
            "user_agent" varchar(255) not null,
            "ip" INET not null,
            "user_id" int not null REFERENCES "user" (id) ON DELETE CASCADE
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
        "#;

        db.execute_unprepared(statement).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(String::from("cannot be reverted")))
    }
}
