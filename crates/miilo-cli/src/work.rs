use crate::args::parse_target;
use miilo_adapter_sdk::WorkItem;
use miilo_client::MiiloClient;

pub async fn find_work(
    client: &MiiloClient,
    target: &str,
) -> Result<WorkItem, Box<dyn std::error::Error>> {
    let (provider_id, work_item_id) = parse_target(target).map_err(std::io::Error::other)?;
    client
        .snapshot()
        .await?
        .work_items
        .into_iter()
        .find(|item| item.provider_id == provider_id && item.id == work_item_id)
        .ok_or_else(|| format!("work item {target} was not found").into())
}
