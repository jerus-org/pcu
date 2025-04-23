mod create_label;
mod get_label_id;
mod get_open_prs;
mod get_pr_id;
mod get_pr_title;
mod get_repo_id;
mod get_tag;
mod label_pr;

pub(crate) use create_label::GraphQLCreateLabel;
pub(crate) use get_label_id::GraphQLGetLabel;
pub(crate) use get_open_prs::GraphQLGetOpenPRs;
pub(crate) use get_pr_id::GraphQLGetPRId;
pub(crate) use get_pr_title::get_pull_request_title;
pub(crate) use get_repo_id::GraphQLGetRepoID;
pub(crate) use get_tag::GraphQLGetTag;
pub(crate) use label_pr::GraphQLLabelPR;
