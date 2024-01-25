use super::constants::Permission;
use super::dto::{self, OrganizationDto, UserDto};
use super::jwt::{self, Claims};
use crate::database::models;
use crate::database::schema::{access_level, organization, session, user};
use crate::modules::auth::session::{SessionId, SESSION_DAYS_DURATION};
use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use diesel_async::scoped_futures::ScopedFutureExt;
use ipnetwork::IpNetwork;
use rand_chacha::ChaCha8Rng;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

pub enum UserFromCredentialsError {
    NotFound,
    InternalError,
    InvalidPassword,
}

#[derive(Clone)]
pub struct AuthService {
    rng: Arc<Mutex<ChaCha8Rng>>,
    db: DatabaseConnection,
}

impl AuthService {
    pub fn new(db: DatabaseConnection, rng: ChaCha8Rng) -> Self {
        AuthService {
            db,
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    /// generates a new session token and creates a new session record on the DB for the user
    pub async fn new_session(
        &self,
        user_identifier: i32,
        client_ip: IpAddr,
        client_user_agent: String,
    ) -> Result<SessionId> {
        let ses_token = SessionId::generate_new(&mut self.rng.lock().unwrap());

        let new_session = entity::session::ActiveModel {
            ip: Set(IpNetwork::from(client_ip).to_string()),
            user_agent: Set(client_user_agent),
            expires_at: Set((Utc::now() + Duration::days(SESSION_DAYS_DURATION)).into()),
            user_id: Set(user_identifier),
            session_token: Set(ses_token.into_database_value()),
            ..Default::default()
        };

        new_session.insert(&self.db).await?;

        Ok(ses_token)
    }

    /// lists all sessions belonging to a user
    pub async fn get_active_user_sessions(
        &self,
        user_id: i32,
    ) -> Result<Vec<(entity::session::Model, entity::user::Model)>> {
        // TODO: !        //     .inner_join(user::table)

        // TODO: corrigir sessao -> user para One to One
        let sessions = entity::session::Entity::find()
            .filter(entity::session::Column::ExpiresAt.gt(Utc::now()))
            .filter(entity::session::Column::UserId.eq(user_id))
            .find_with_related(entity::user::Entity)
            .all(&self.db)
            .await?;

        Ok(sessions)
    }

    /// deletes a session by its token
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        use crate::database::schema::session::dsl::*;

        let conn = &mut self.db_conn_pool.get().await?;

        let delete_query = session.filter(session_token.eq(session_id.into_database_value()));
        diesel::delete(delete_query).execute(conn).await?;

        Ok(())
    }

    /// gets the user from the session token if the session is not expired
    pub async fn get_user_from_session_id(
        &self,
        token: SessionId,
    ) -> Result<Option<UserDtoEntities>> {
        let conn = &mut self.db_conn_pool.get().await?;

        Ok(user::table
            .inner_join(session::table)
            .inner_join(access_level::table)
            .left_join(organization::table)
            .filter(session::dsl::session_token.eq(token.into_database_value()))
            .filter(session::dsl::expires_at.gt(Utc::now()))
            .select((
                models::User::as_select(),
                models::AccessLevel::as_select(),
                Option::<models::Organization>::as_select(),
            ))
            .first::<UserDtoEntities>(conn)
            .await
            .optional()?)
    }

    /// finds a user from email and plain text password, verifying the password
    pub async fn get_user_from_credentials(
        &self,
        user_email: String,
        user_password: String,
    ) -> Result<dto::UserDto, UserFromCredentialsError> {
        let conn = &mut self
            .db_conn_pool
            .get()
            .await
            .or(Err(UserFromCredentialsError::InternalError))?;

        let user_model: Option<UserDtoEntities> = user::table
            .inner_join(access_level::table)
            .left_join(organization::table)
            .filter(user::email.eq(&user_email))
            .select((
                models::User::as_select(),
                models::AccessLevel::as_select(),
                Option::<models::Organization>::as_select(),
            ))
            .first::<UserDtoEntities>(conn)
            .await
            .optional()
            .or(Err(UserFromCredentialsError::InternalError))?;

        match user_model {
            Some(usr) => {
                let pass_is_valid = verify(user_password, &usr.0.password)
                    .or(Err(UserFromCredentialsError::InternalError))?;

                if pass_is_valid {
                    Ok(UserDto::from(usr))
                } else {
                    Err(UserFromCredentialsError::InvalidPassword)
                }
            }
            None => Err(UserFromCredentialsError::NotFound),
        }
    }

    /// checks if a email is in use by a organization or a user
    pub async fn check_email_in_use(&self, email: &str) -> Result<bool> {
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

        let user_id: Option<i32> = user::dsl::user
            .select(user::dsl::id)
            .filter(user::dsl::email.eq(&email))
            .first(conn)
            .await
            .optional()?;

        Ok(user_id.is_some())
    }

    pub async fn get_user_id_by_username(&self, username: &str) -> Result<Option<i32>> {
        let conn = &mut self.db_conn_pool.get().await?;

        let user_id: Option<i32> = user::dsl::user
            .select(user::dsl::id)
            .filter(user::dsl::username.eq(&username))
            .first(conn)
            .await
            .optional()?;

        Ok(user_id)
    }

    pub async fn gen_and_set_user_reset_password_token(&self, user_id: i32) -> Result<String> {
        use crate::database::schema::user::dsl::*;

        let mut claims = Claims::default();

        claims.set_expiration_in(Duration::minutes(15));
        claims.aud = format!("user:{}", user_id);
        claims.sub = String::from("restore password token");

        let token = jwt::encode(&claims)?;

        let conn = &mut self.db_conn_pool.get().await?;

        diesel::update(user)
            .filter(id.eq(user_id))
            .set(reset_password_token.eq(&token))
            .execute(conn)
            .await?;

        Ok(token)
    }

    pub async fn gen_and_set_user_confirm_email_token(&self, user_id: i32) -> Result<String> {
        use crate::database::schema::user::dsl::*;

        let mut claims = Claims::default();

        claims.set_expiration_in(Duration::hours(8));
        claims.aud = format!("user:{}", user_id);
        claims.sub = String::from("confirm email address token");

        let token = jwt::encode(&claims)?;

        let conn = &mut self.db_conn_pool.get().await?;

        diesel::update(user)
            .filter(id.eq(user_id))
            .set(confirm_email_token.eq(&token))
            .execute(conn)
            .await?;

        Ok(token)
    }

    pub async fn gen_and_set_org_confirm_email_token(&self, org_id: i32) -> Result<String> {
        use crate::database::schema::organization::dsl::*;

        let mut claims = Claims::default();

        claims.set_expiration_in(Duration::hours(8));
        claims.aud = format!("organization:{}", org_id);
        claims.sub = String::from("confirm email address token");

        let token = jwt::encode(&claims)?;

        let conn = &mut self.db_conn_pool.get().await?;

        diesel::update(organization)
            .filter(id.eq(org_id))
            .set(confirm_billing_email_token.eq(&token))
            .execute(conn)
            .await?;

        Ok(token)
    }

    /// creates a new user and his organization, as well as a root access level for said org
    pub async fn register_user_and_organization(
        &self,
        dto: dto::RegisterOrganization,
    ) -> Result<dto::UserDto> {
        let conn = &mut self.db_conn_pool.get().await?;

        let user_dto = conn
            .transaction::<_, anyhow::Error, _>(|conn| {
                async move {
                    let created_organization = diesel::insert_into(organization::dsl::organization)
                        .values((
                            organization::dsl::name.eq(&dto.username),
                            organization::dsl::blocked.eq(false),
                            organization::dsl::billing_email.eq(&dto.email),
                            organization::dsl::billing_email_verified.eq(false),
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

                    let created_user = diesel::insert_into(user::dsl::user)
                        .values((
                            user::dsl::email.eq(dto.email),
                            user::dsl::username.eq(dto.username),
                            user::dsl::password.eq(hash(dto.password, DEFAULT_COST)?),
                            user::dsl::email_verified.eq(false),
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

                    Ok(UserDto::from((
                        created_user,
                        created_access_level,
                        Some(created_organization),
                    )))
                }
                .scope_boxed()
            })
            .await?;

        Ok(user_dto)
    }
}

/// tuple with relevant relationships (`access_level` and `organization`) to create a user dto
pub type UserDtoEntities = (
    models::User,
    models::AccessLevel,
    Option<models::Organization>,
);

impl From<UserDtoEntities> for UserDto {
    fn from(m: UserDtoEntities) -> Self {
        let (user, access_level, org) = m;

        Self {
            id: user.id,
            created_at: user.created_at,
            username: user.username,
            email: user.email,
            email_verified: user.email_verified,
            profile_picture: user.profile_picture,
            description: user.description,
            organization: org.map(|o| OrganizationDto::from(o)),
            access_level: Into::into(access_level),
        }
    }
}
