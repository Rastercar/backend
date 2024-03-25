use sea_orm::{DatabaseConnection, DbErr};

/// Trait for entities that can be queried by their ID and a org ID
///
/// this is mostly usefull for entities that are always bound to
/// a organization and we need to check verify the organization of
/// the entity is the same as the organization of whos querying for it.
pub trait QueryableByIdAndOrgId {
    /// The model of the entity that is returned by the query
    type Model;

    fn find_by_id_and_org_id(
        id: i32,
        org_id: i32,
        db: &DatabaseConnection,
    ) -> impl std::future::Future<Output = Result<Option<Self::Model>, DbErr>> + Send;
}
