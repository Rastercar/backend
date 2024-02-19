use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let statement = r#"
CREATE TYPE "tracker_model" AS ENUM ('H02');

CREATE TABLE "organization" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "name" varchar(255) NOT NULL,
    "blocked" boolean NOT NULL,
    "billing_email" varchar(255) NOT NULL,
    "billing_email_verified" boolean NOT NULL DEFAULT FALSE,
    "confirm_billing_email_token" TEXT NULL,
    "owner_id" int NULL
);

ALTER TABLE "organization"
ADD CONSTRAINT "organization_billing_email_unique" UNIQUE ("billing_email");

ALTER TABLE "organization"
ADD CONSTRAINT "organization_owner_id_unique" UNIQUE ("owner_id");

CREATE TABLE "user" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "username" varchar(255) NOT NULL,
    "email" varchar(255) NOT NULL,
    "email_verified" boolean NOT NULL DEFAULT FALSE,
    "password" varchar(255) NOT NULL,
    "reset_password_token" TEXT NULL,
    "confirm_email_token" TEXT NULL,
    "profile_picture" varchar(255) NULL,
    "description" TEXT NULL,
    "organization_id" int,
    "access_level_id" int NOT NULL
);

ALTER TABLE "user"
ADD CONSTRAINT "user_email_unique" UNIQUE ("email");

ALTER TABLE "user"
ADD CONSTRAINT "user_username_unique" UNIQUE ("username");

ALTER TABLE "user"
ADD CONSTRAINT "user_reset_password_token_unique" UNIQUE ("reset_password_token");

ALTER TABLE "user"
ADD CONSTRAINT "user_confirm_email_token_unique" UNIQUE ("confirm_email_token");

CREATE TABLE "access_level" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "name" varchar(255) NOT NULL,
    "description" TEXT NOT NULL,
    "is_fixed" boolean NOT NULL,
    "permissions" TEXT [] NOT NULL DEFAULT '{}',
    "organization_id" int NULL
);

CREATE TABLE "vehicle" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "plate" varchar(255) NOT NULL,
    "photo" varchar(255) NULL,
    "model_year" SMALLINT NULL,
    "fabrication_year" SMALLINT NULL,
    "chassis_number" varchar(255) NULL,
    "brand" varchar(255) NULL,
    "model" varchar(255) NULL,
    "color" varchar(255) NULL,
    "additional_info" varchar(255) NULL,
    "organization_id" int NOT NULL
);

ALTER TABLE "vehicle"
ADD CONSTRAINT "vehicle_plate_unique" UNIQUE ("plate", "organization_id");

CREATE TABLE "vehicle_tracker" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "model" tracker_model NOT NULL,
    "imei" varchar(255) NOT NULL,
    "organization_id" int NOT NULL,
    "vehicle_id" int NULL
);

ALTER TABLE "vehicle_tracker"
ADD CONSTRAINT "vehicle_tracker_imei_unique" UNIQUE ("imei", "organization_id");

ALTER TABLE "vehicle_tracker"
ADD CONSTRAINT "vehicle_tracker_vehicle_id_unique" UNIQUE ("vehicle_id");

CREATE TABLE "sim_card" (
    "id" serial PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "phone_number" varchar(255) NOT NULL,
    "ssn" varchar(255) NOT NULL,
    "apn_address" varchar(255) NOT NULL,
    "apn_user" varchar(255) NOT NULL,
    "apn_password" varchar(255) NOT NULL,
    "pin" varchar(8) NULL,
    "pin2" varchar(8) NULL,
    "puk" varchar(8) NULL,
    "puk2" varchar(8) NULL,
    "organization_id" int NOT NULL,
    "vehicle_tracker_id" int NULL
);

COMMENT ON
COLUMN "sim_card"."phone_number" IS 'Phone numbers are stored in the E164 international format';

ALTER TABLE "sim_card"
ADD CONSTRAINT "sim_card_phone_number_unique" UNIQUE ("phone_number", "organization_id");

ALTER TABLE "sim_card"
ADD CONSTRAINT "sim_card_ssn_unique" UNIQUE ("ssn", "organization_id");

CREATE TABLE "vehicle_tracker_last_location" (
    "vehicle_tracker_id" int NOT NULL,
    "time" timestamptz(0) NOT NULL,
    "point" geometry NOT NULL,
    CONSTRAINT "vehicle_tracker_last_location_pkey" PRIMARY KEY ("vehicle_tracker_id")
);

ALTER TABLE "vehicle_tracker_last_location"
ADD CONSTRAINT "vehicle_tracker_last_location_vehicle_tracker_id_unique" UNIQUE ("vehicle_tracker_id");

CREATE TABLE "vehicle_tracker_location" (
    "time" timestamptz(0) NOT NULL,
    "vehicle_tracker_id" int NOT NULL,
    "point" geometry NOT NULL,
    CONSTRAINT "vehicle_tracker_location_pkey" PRIMARY KEY ("time", "vehicle_tracker_id")
);

CREATE TABLE "session" (
    "public_id" serial UNIQUE,
    "session_token" BYTEA PRIMARY KEY,
    "created_at" timestamptz(0) NOT NULL DEFAULT now(),
    "expires_at" timestamptz(0) NOT NULL,
    "user_agent" varchar(255) NOT NULL,
    "ip" INET NOT NULL,
    "user_id" int NOT NULL REFERENCES "user" (id) ON DELETE CASCADE
);

ALTER TABLE "organization"
ADD CONSTRAINT "organization_owner_id_foreign" FOREIGN KEY ("owner_id") REFERENCES "user" ("id")
ON UPDATE CASCADE
ON DELETE SET NULL;

ALTER TABLE "user"
ADD CONSTRAINT "user_organization_id_foreign" FOREIGN KEY ("organization_id") REFERENCES "organization" ("id")
ON UPDATE CASCADE;

ALTER TABLE "user"
ADD CONSTRAINT "user_access_level_id_foreign" FOREIGN KEY ("access_level_id") REFERENCES "access_level" ("id")
ON UPDATE CASCADE;

ALTER TABLE "access_level"
ADD CONSTRAINT "access_level_organization_id_foreign" FOREIGN KEY ("organization_id") REFERENCES "organization" ("id")
ON UPDATE CASCADE
ON DELETE SET NULL;

ALTER TABLE "vehicle"
ADD CONSTRAINT "vehicle_organization_id_foreign" FOREIGN KEY ("organization_id") REFERENCES "organization" ("id")
ON UPDATE CASCADE;

ALTER TABLE "vehicle_tracker"
ADD CONSTRAINT "vehicle_tracker_organization_id_foreign" FOREIGN KEY ("organization_id") REFERENCES "organization" ("id") 
ON UPDATE CASCADE;

ALTER TABLE "vehicle_tracker"
ADD CONSTRAINT "vehicle_tracker_vehicle_id_foreign" FOREIGN KEY ("vehicle_id") REFERENCES "vehicle" ("id")
ON UPDATE CASCADE 
ON DELETE SET NULL;

ALTER TABLE "sim_card"
ADD CONSTRAINT "sim_card_organization_id_foreign" FOREIGN KEY ("organization_id") REFERENCES "organization" ("id") 
ON UPDATE CASCADE;

ALTER TABLE "sim_card"
ADD CONSTRAINT "sim_card_vehicle_tracker_id_foreign" FOREIGN KEY ("vehicle_tracker_id") REFERENCES "vehicle_tracker" ("id")
ON UPDATE CASCADE
ON DELETE SET NULL;

ALTER TABLE "vehicle_tracker_last_location"
ADD CONSTRAINT "vehicle_tracker_last_location_vehicle_tracker_id_foreign" FOREIGN KEY ("vehicle_tracker_id") REFERENCES "vehicle_tracker" ("id")
ON UPDATE CASCADE
ON DELETE CASCADE;
        "#;

        db.execute_unprepared(statement).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(String::from("cannot be reverted")))
    }
}
