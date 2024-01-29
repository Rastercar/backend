use entity::{access_level, organization, user};
use fake::{faker, Fake};
use sea_orm_migration::{
    sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set},
    sea_query::Expr,
    DbErr,
};
use shared::Permission;

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

    Ok(())
}
