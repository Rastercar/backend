use super::models::User;
use super::schema::user;
use diesel::dsl::Filter;
use diesel::dsl::{AsSelect, Select};
use diesel::query_dsl::QueryDsl;
use diesel::{ExpressionMethods, SelectableHelper};

type UserAll = Select<user::table, AsSelect<User, diesel::pg::Pg>>;

impl User {
    pub fn all() -> UserAll {
        user::table.select(User::as_select())
    }

    pub fn by_email(email: &str) -> Filter<UserAll, diesel::dsl::Eq<user::email, &str>> {
        Self::all().filter(user::email.eq(email))
    }
}
