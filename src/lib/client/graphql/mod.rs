mod pr_title;
mod prs;

pub(crate) use pr_title::get_pull_request_title;
use prs::PrItem;

use crate::Error;

pub(crate) trait GraphQL {
    #[allow(async_fn_in_trait)]
    async fn get_open_pull_requests(&self) -> Result<Vec<PrItem>, Error>;
}
