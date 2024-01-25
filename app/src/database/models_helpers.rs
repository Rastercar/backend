use super::models::{User, Vehicle};
use super::schema::{user, vehicle};
use diesel::dsl::Filter;
use diesel::dsl::{AsSelect, Select};
use diesel::query_dsl::QueryDsl;
use diesel::{ExpressionMethods, SelectableHelper};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

type UserAll = Select<user::table, AsSelect<User, diesel::pg::Pg>>;

pub type DbConn = deadpool::managed::Object<AsyncDieselConnectionManager<AsyncPgConnection>>;

// https://diesel.rs/guides/composing-applications.html
impl User {
    pub fn all() -> UserAll {
        user::table.select(User::as_select())
    }

    // [PROD-TODO] this is here just to show a quick example on how to compose
    // applications if this is not used repeatedly this can be removed
    pub fn by_email(email: &str) -> Filter<UserAll, diesel::dsl::Eq<user::email, &str>> {
        Self::all().filter(user::email.eq(email))
    }
}

impl Vehicle {
    pub async fn delete_self(&self, conn: &mut DbConn) -> Result<usize, diesel::result::Error> {
        diesel::delete(vehicle::dsl::vehicle)
            .filter(vehicle::dsl::id.eq(self.id))
            .execute(conn)
            .await
    }

    pub async fn set_photo(
        &mut self,
        conn: &mut DbConn,
        photo: Option<String>,
    ) -> Result<usize, diesel::result::Error> {
        let update_result = diesel::update(vehicle::dsl::vehicle)
            .filter(vehicle::dsl::id.eq(self.id))
            .set(vehicle::dsl::photo.eq(&photo))
            .execute(conn)
            .await;

        if !update_result.is_err() {
            self.photo = photo
        }

        update_result
    }
}
