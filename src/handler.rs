use crate::{Context};
use serde_json::json;
use crate::repository::Repository;


pub async fn get_contact(ctx: Context) -> String {
    let id = match ctx.params.find("id") {
        Some(v) => v,
        None => "empty",
    }.parse().unwrap();
    let contact = ctx.state.repository.get(id).await.unwrap();
    json!(&contact).to_string()
}
