use crate::meta::search::Response;
pub enum SearchResultType {
    Cached(Response),
    Search(Response),
}
