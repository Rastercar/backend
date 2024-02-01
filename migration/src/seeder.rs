use entity::{access_level, organization, sim_card, user, vehicle, vehicle_tracker};
use fake::{faker, Fake};
use rand::{seq::SliceRandom, Rng};
use sea_orm_migration::{
    sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set},
    sea_query::Expr,
    DbErr,
};
use shared::Permission;

use crate::seeder_consts;

const ALPHA: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMERIC: &str = "0123456789";

/// Hash a password with bcrypt using the lowest cost (4)
/// since we do not care about security of seeded data
fn hash_password(plain: String) -> String {
    bcrypt::hash(plain, 4).unwrap().to_string()
}

fn fake_password() -> String {
    hash_password(faker::internet::en::Password(10..50).fake::<String>())
}

fn fake_words(range: std::ops::Range<usize>) -> String {
    faker::lorem::en::Words(range)
        .fake::<Vec<String>>()
        .join(" ")
}

/// Creates a brazilian vehicle plate in the `AAA9999` format, where:
///
/// - A = uppercase alphabetic characters
/// - 9 = numbers 0 to 9
fn fake_br_vehicle_plate() -> String {
    let a: String = fake::StringFaker::with(Vec::from(ALPHA), 3).fake();
    let b: String = fake::StringFaker::with(Vec::from(NUMERIC), 4).fake();

    a.to_string() + b.as_str()
}

fn fake_imei() -> String {
    fake::StringFaker::with(Vec::from(ALPHA), 20).fake()
}

/// Creates a random boolean with a certain % of chance to be `true`
fn fake_bool_with_chance(chance_to_be_true: u8) -> bool {
    let n = rand::thread_rng().gen_range(0..100);

    n < chance_to_be_true
}

/// Creates a random SIM card PIN (personal identification number)
///
/// see: https://www.sciencedirect.com/topics/computer-science/personal-identification-number
fn fake_pin_number() -> String {
    rand::thread_rng().gen_range(1000..9999).to_string()
}

/// Creates a random SIM card PUK (personal unlocking key)
///
/// see: https://www.sciencedirect.com/topics/computer-science/personal-identification-number
fn fake_puk_code() -> String {
    rand::thread_rng().gen_range(10000..999999).to_string()
}

/// Creates a random SIM card SSN
fn fake_sim_ssn() -> String {
    format!(
        "00{}",
        rand::thread_rng().gen_range(10000..999999).to_string()
    )
}

pub async fn organization(db: &DatabaseTransaction) -> Result<organization::Model, DbErr> {
    let org = organization::ActiveModel {
        name: Set(faker::company::en::CompanyName().fake::<String>()),
        blocked: Set(false),
        billing_email: Set(faker::internet::en::SafeEmail().fake::<String>()),
        billing_email_verified: Set(true),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(org)
}

pub async fn tracker(
    db: &DatabaseTransaction,
    org_id: i32,
    vehicle_id: Option<i32>,
) -> Result<vehicle_tracker::Model, DbErr> {
    let t = vehicle_tracker::ActiveModel {
        model: Set(shared::TrackerModel::H02.to_string()),
        imei: Set(fake_imei()),
        vehicle_id: Set(vehicle_id),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(t)
}

pub async fn sim_card(
    db: &DatabaseTransaction,
    org_id: i32,
    tracker_id: Option<i32>,
) -> Result<sim_card::Model, DbErr> {
    let apn = seeder_consts::get_fake_apn();

    let phone: String = faker::phone_number::en::CellNumber().fake();

    let t = sim_card::ActiveModel {
        phone_number: Set(phone),
        ssn: Set(fake_sim_ssn()),
        apn_user: Set(apn.user),
        apn_address: Set(apn.apn),
        apn_password: Set(apn.pass),
        puk: Set(Some(fake_puk_code())),
        puk2: Set(Some(fake_puk_code())),
        pin: Set(Some(fake_pin_number())),
        pin2: Set(Some(fake_pin_number())),
        tracker_id: Set(tracker_id),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(t)
}

pub async fn vehicle(db: &DatabaseTransaction, org_id: i32) -> Result<vehicle::Model, DbErr> {
    let color = seeder_consts::COLORS
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string();

    let brand = seeder_consts::CAR_BRANDS
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string();

    // we dont care if the model does not belong to the brand, seeded data can be silly
    let model = seeder_consts::VEHICLE_MODELS
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string();

    let fabrication_year = rand::thread_rng().gen_range(2000..2024);

    let v = vehicle::ActiveModel {
        plate: Set(fake_br_vehicle_plate()),
        model_year: Set(Some(fabrication_year + 1)),
        fabrication_year: Set(Some(fabrication_year)),
        brand: Set(Some(brand)),
        color: Set(Some(color)),
        model: Set(Some(model)),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(v)
}

pub async fn access_level(
    db: &DatabaseTransaction,
    is_fixed: bool,
    org_id: Option<i32>,
) -> Result<access_level::Model, DbErr> {
    let lev = access_level::ActiveModel {
        name: Set(faker::lorem::en::Word().fake::<String>()),
        is_fixed: Set(is_fixed),
        description: Set(fake_words(2..7)),
        permissions: Set(Permission::to_string_vec()),
        organization_id: Set(org_id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(lev)
}

pub async fn test_master_user(db: &DatabaseTransaction) -> Result<user::Model, DbErr> {
    let test_master_user_access_level = access_level(db, true, None).await?;

    let u = user::ActiveModel {
        username: Set(String::from("test_master_user")),
        password: Set(hash_password(String::from("testmasteruser"))),

        email: Set(String::from("rastercar.tests.001@gmail.com")),
        email_verified: Set(true),
        access_level_id: Set(test_master_user_access_level.id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(u)
}

pub async fn test_user(db: &DatabaseTransaction) -> Result<user::Model, DbErr> {
    let test_user_organization = organization::ActiveModel {
        name: Set(String::from("test user org")),
        blocked: Set(false),
        billing_email: Set(String::from("testuser@gmail.com")),
        billing_email_verified: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let test_user_access_level = access_level(db, true, Some(test_user_organization.id)).await?;

    let u = user::ActiveModel {
        email: Set(String::from("rastercar.tests.002@gmail.com")),
        email_verified: Set(true),
        username: Set(String::from("test_user")),
        password: Set(hash_password(String::from("testuser"))),
        description: Set(Some(fake_words(1..3))),
        organization_id: Set(Some(test_user_organization.id)),
        access_level_id: Set(test_user_access_level.id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(u)
}

pub async fn create_entities_for_org(db: &DatabaseTransaction, org_id: i32) -> Result<(), DbErr> {
    for _ in 0..20 {
        let vehicle = vehicle(db, org_id).await?;

        // for 75% of the vehicles, create a tracker and possibly its SIM card(s)
        if fake_bool_with_chance(75) {
            let tracker = tracker(db, org_id, Some(vehicle.id)).await?;

            // the tracker has a 80% chance of having a SIM CARD
            if fake_bool_with_chance(80) {
                sim_card(db, org_id, Some(tracker.id)).await?;

                // the tracker has a 8% chance (0.8 * 0.1) of having a second sim card
                // (some tracker models can have 2 sim cards in case one looses connection)
                if fake_bool_with_chance(10) {
                    sim_card(db, org_id, Some(tracker.id)).await?;
                }
            }
        }
    }

    // create some trackers that are not associated with a vehicle
    for _ in 0..5 {
        tracker(db, org_id, None).await?;
    }

    // create some SIM cards that are not associated with trackers
    for _ in 0..5 {
        sim_card(db, org_id, None).await?;
    }

    Ok(())
}

pub async fn root_user_with_user_org(db: &DatabaseTransaction) -> Result<(), DbErr> {
    let user_org = organization(db).await?;
    let access_level = access_level(db, true, Some(user_org.id)).await?;

    let created_user = user::ActiveModel {
        email_verified: Set(faker::boolean::en::Boolean(50).fake::<bool>()),
        username: Set(faker::internet::en::Username().fake::<String>()),
        password: Set(fake_password()),
        email: Set(faker::internet::en::SafeEmail().fake::<String>()),
        organization_id: Set(Some(user_org.id)),
        access_level_id: Set(access_level.id),
        description: Set(Some(fake_words(1..3))),
        ..Default::default()
    }
    .insert(db)
    .await?;

    user::ActiveModel {
        email_verified: Set(faker::boolean::en::Boolean(50).fake::<bool>()),
        username: Set(faker::internet::en::Username().fake::<String>()),
        password: Set(fake_password()),
        email: Set(faker::internet::en::SafeEmail().fake::<String>()),
        description: Set(Some(fake_words(1..3))),

        organization_id: Set(Some(user_org.id)),
        access_level_id: Set(access_level.id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    organization::Entity::update_many()
        .col_expr(organization::Column::OwnerId, Expr::value(created_user.id))
        .filter(organization::Column::Id.eq(user_org.id))
        .exec(db)
        .await?;

    create_entities_for_org(db, user_org.id).await?;

    Ok(())
}
