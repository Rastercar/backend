use sea_orm::{ActiveValue, Paginator, SelectorTrait};
use utoipa::ToSchema;

use crate::modules::common::dto::{Pagination, PaginationResult};

use super::error::DbError;

/// Executes a paginated query, fetching its items, number of items and number
/// of pages into a `PaginationResult`
pub async fn paginated_query_to_pagination_result<
    'db,
    C: sea_orm::ConnectionTrait,
    S: sea_orm::SelectorTrait,
>(
    paginator: Paginator<'db, C, S>,
    pagination: Pagination,
) -> Result<PaginationResult<S::Item>, DbError>
where
    for<'_s> <S as SelectorTrait>::Item: ToSchema<'_s>,
{
    let n = paginator.num_items_and_pages().await?;
    let records = paginator.fetch_page(pagination.page - 1).await?;

    let result = PaginationResult {
        page: pagination.page,
        records,
        page_size: pagination.page_size,
        item_count: n.number_of_items,
        page_count: n.number_of_pages,
    };

    Ok(result)
}

/// if opt is `None` returns `ActiveValue::NotSet` otherwise
/// returns `ActiveValue::Set(v)`. This is usefull for to
/// conditionally change a `ActiveModel` value.
///
/// eg: whenever parsing JSON with optional fields, its common to use `Option<Option<T>>`
/// to differ between undefined and NULL, where:
/// - `None` means undefined
/// - `Some(None)` means NULL
/// - `Some(Some(v))` means a value
///
/// so to set nullable and possibly undefined field to NULL if the value is defined we could
/// use this function as follows
///
/// ```
/// let null_description: Option<Option<String>> = Some(None)
/// let undefined_description: Option<Option<String>> = None
///
/// // sets user description to NULL
/// user.description = set_if_some(null_description)
///
/// // does not change user description, keeping it as ActiveValue::NotSet
/// user.description = set_if_some(undefined_description)
/// ```
pub fn set_if_some<T>(opt: Option<T>) -> ActiveValue<T>
where
    sea_orm::Value: From<T>,
{
    if let Some(v) = opt {
        ActiveValue::Set(v)
    } else {
        ActiveValue::NotSet
    }
}
