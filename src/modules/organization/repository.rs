use crate::database::models;
use crate::database::schema::{organization, unregistered_user};
use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

#[derive(Clone)]
pub struct OrganizationRepository {
    db_conn_pool: Pool<AsyncPgConnection>,
}

pub fn new_organization_repository(
    db_conn_pool: Pool<AsyncPgConnection>,
) -> OrganizationRepository {
    OrganizationRepository { db_conn_pool }
}

impl OrganizationRepository {
    pub async fn create_organization(&self) -> Result<()> {
        let mut conn = self.db_conn_pool.get().await?;

        let data: Vec<models::UnregisteredUser> = unregistered_user::dsl::unregistered_user
            .select(models::UnregisteredUser::as_select())
            .load(&mut conn)
            .await?;

        println!("Displaying {:#?}", data);

        Ok(())
    }

    // TODO: move me to proper file
    pub async fn check_email_in_use(&self, email: String) -> Result<()> {
        let mut conn = self.db_conn_pool.get().await?;

        let data: Result<i32, diesel::result::Error> = organization::dsl::organization
            .select(organization::dsl::id)
            .filter(organization::dsl::billing_email.eq(email))
            .first(&mut conn)
            .optional()
            .await;

        Ok(())
    }
}
