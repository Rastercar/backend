use sea_orm::{Paginator, SelectorTrait};
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
