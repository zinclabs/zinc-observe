use config::meta::search::{SearchEventContext, SearchEventType};

#[inline(always)]
pub(crate) fn get_search_type_from_ws_req(
    search_event_type: &SearchEventType,
    search_event_context: SearchEventContext,
) -> Option<SearchEventContext> {
    match search_event_type {
        SearchEventType::Dashboards => Some(SearchEventContext::with_dashboard(
            search_event_context.dashboard_id,
            search_event_context.dashboard_name,
            search_event_context.dashboard_folder_id,
            search_event_context.dashboard_folder_name,
        )),
        SearchEventType::Alerts => Some(SearchEventContext::with_alert(
            search_event_context.alert_key,
        )),
        SearchEventType::Reports => Some(SearchEventContext::with_report(
            search_event_context.report_key,
        )),
        _ => None,
    }
}