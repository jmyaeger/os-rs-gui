use gloo_net::http::Request;
use js_sys::encode_uri_component;
use osrs::types::player::parse_player_data;
use osrs::types::stats::PlayerStats;

const HISCORES_API_URL: &str = "https://hiscores-proxy.jmyaeger.workers.dev";

pub async fn fetch_player_stats(rsn: &str) -> Result<PlayerStats, String> {
    let encoded_rsn = encode_uri_component(rsn);
    let url = format!("{HISCORES_API_URL}/?player={encoded_rsn}");

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = response.status();
    if status == 404 {
        return Err(format!("Player not found: {rsn}"));
    }

    if status != 200 {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API request failed ({status}): {error_text}"));
    }

    let data = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    parse_player_data(data).map_err(|e| format!("Failed to parse hiscores data: {e}"))
}
