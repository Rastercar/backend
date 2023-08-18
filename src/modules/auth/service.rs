use super::constants::Permission;
use crate::database::models;
use crate::database::schema::{access_level, master_user, organization, unregistered_user, user};
use crate::modules::organization::dto;
use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncConnection, AsyncPgConnection};

#[derive(Clone)]
pub struct AuthService {
    db_conn_pool: Pool<AsyncPgConnection>,
}

pub fn new_auth_service(db_conn_pool: Pool<AsyncPgConnection>) -> AuthService {
    AuthService { db_conn_pool }
}

impl AuthService {
    pub async fn login_for_user(
        &self,
        user_model: models::User,
        set_last_login: bool,
    ) -> Result<(models::User, String)> {
        if set_last_login || user_model.google_profile_id.is_some() {
            let conn = &mut self.db_conn_pool.get().await?;

            if set_last_login {
                use crate::database::schema::user::dsl::*;
                diesel::update(user)
                    .filter(id.eq(user_model.id))
                    .set(last_login.eq(Utc::now()))
                    .execute(conn);
            }

            if let Some(g_profile_id) = &user_model.google_profile_id {
                use crate::database::schema::unregistered_user::dsl::*;

                let delete_query = unregistered_user
                    .filter(oauth_profile_id.eq(g_profile_id))
                    .filter(oauth_provider.eq("google"));

                diesel::delete(delete_query).execute(conn).await?;
            }
        }

        // TODO: !
        // const token = this.authTokenService.createTokenForUser(user, options.tokenOptions)
        // sub: 'user'-${user.id}
        // jwtService.sign({ sub }, options)

        Ok((user_model, String::from("")))
    }

    /// checks if a email is in use by a organization, master user or a user
    pub async fn check_email_in_use(&self, email: String) -> Result<bool> {
        let conn = &mut self.db_conn_pool.get().await?;

        let organization_id: Option<i32> = organization::dsl::organization
            .select(organization::dsl::id)
            .filter(organization::dsl::billing_email.eq(&email))
            .first(conn)
            .await
            .optional()?;

        if organization_id.is_some() {
            return Ok(true);
        }

        let master_user_id: Option<i32> = master_user::dsl::master_user
            .select(master_user::dsl::id)
            .filter(master_user::dsl::email.eq(&email))
            .first(conn)
            .await
            .optional()?;

        if master_user_id.is_some() {
            return Ok(true);
        }

        let user_id: Option<i32> = user::dsl::user
            .select(user::dsl::id)
            .filter(user::dsl::email.eq(&email))
            .first(conn)
            .await
            .optional()?;

        return Ok(user_id.is_some());
    }

    /// creates a new user and his organization, as well as a root access level for said org
    /// and finally deletes the unregistered user record if the user being registered refers
    /// to a previously unregistered user
    pub async fn register_user_and_organization(
        &self,
        dto: dto::RegisterUser,
    ) -> Result<models::User> {
        let conn = &mut self.db_conn_pool.get().await?;

        let unregistered_user_finishing_registration: Option<models::UnregisteredUser> =
            match dto.refers_to_unregistered_user {
                None => None,
                Some(ur_user_id) => unregistered_user::dsl::unregistered_user
                    .find(ur_user_id)
                    .select(models::UnregisteredUser::as_select())
                    .first(conn)
                    .await
                    .optional()?,
            };

        let email_verified = match &unregistered_user_finishing_registration {
            Some(u) => u.email_verified,
            None => false,
        };

        let created_user = conn
            .transaction::<_, anyhow::Error, _>(|conn| {
                async move {
                    let created_organization = diesel::insert_into(organization::dsl::organization)
                        .values((
                            organization::dsl::name.eq(&dto.username),
                            organization::dsl::blocked.eq(false),
                            organization::dsl::billing_email.eq(&dto.email),
                            organization::dsl::billing_email_verified.eq(email_verified),
                        ))
                        .get_result::<models::Organization>(conn)
                        .await?;

                    let created_access_level = diesel::insert_into(access_level::dsl::access_level)
                        .values((
                            access_level::dsl::name.eq("admin"),
                            access_level::dsl::is_fixed.eq(true),
                            access_level::dsl::description.eq("root access level"),
                            access_level::dsl::organization_id.eq(created_organization.id),
                            access_level::dsl::permissions.eq(Permission::to_string_vec()),
                        ))
                        .get_result::<models::AccessLevel>(conn)
                        .await?;

                    let google_profile_id = match &unregistered_user_finishing_registration {
                        Some(u) => {
                            if u.oauth_provider == "google" {
                                Some(u.oauth_profile_id.clone())
                            } else {
                                None
                            }
                        }
                        None => None,
                    };

                    if let Some(ur_user) = unregistered_user_finishing_registration {
                        let delete_query = unregistered_user::dsl::unregistered_user
                            .filter(unregistered_user::dsl::uuid.eq(ur_user.uuid));

                        diesel::delete(delete_query).execute(conn).await?;
                    }

                    let created_user = diesel::insert_into(user::dsl::user)
                        .values((
                            user::dsl::email.eq(dto.email),
                            user::dsl::username.eq(dto.username),
                            user::dsl::password.eq(hash(dto.password, DEFAULT_COST)?),
                            user::dsl::email_verified.eq(email_verified),
                            user::dsl::google_profile_id.eq(google_profile_id),
                            user::dsl::organization_id.eq(created_organization.id),
                            user::dsl::access_level_id.eq(created_access_level.id),
                        ))
                        .get_result::<models::User>(conn)
                        .await?;

                    diesel::update(organization::dsl::organization)
                        .filter(organization::dsl::id.eq(created_organization.id))
                        .set(organization::dsl::owner_id.eq(created_user.id))
                        .execute(conn)
                        .await?;

                    Ok(created_user)
                }
                .scope_boxed()
            })
            .await?;

        Ok(created_user)
    }
}
