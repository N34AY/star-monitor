use serde::Serialize;
use tonic::transport::Channel;

use crate::pb::space_x::api::device::device_client::DeviceClient;
use crate::pb::space_x::api::device::request;
use crate::pb::space_x::api::device::response;
use crate::pb::space_x::api::device::{Request as DeviceRequest, Response as DeviceResponse};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSection {
    pub key: String,
    pub title: String,
    pub response_field: String,
    pub status: String,
    pub ok: bool,
    pub raw: String,
    pub error: Option<String>,
}

pub async fn connect_client(dish_address: &str) -> Result<DeviceClient<Channel>, String> {
    DeviceClient::connect(dish_address.to_string())
        .await
        .map_err(|e| format!("failed to connect to {dish_address}: {e}"))
}

pub async fn run_section(
    client: &mut DeviceClient<Channel>,
    key: &str,
    title: &str,
    request_variant: request::Request,
    expected_field: &str,
) -> DataSection {
    match call_handle(client, request_variant).await {
        Ok(response) => {
            let got_field = response
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != expected_field {
                return DataSection {
                    key: key.to_string(),
                    title: title.to_string(),
                    response_field: got_field,
                    status: "error".to_string(),
                    ok: false,
                    raw: String::new(),
                    error: Some(format!(
                        "unexpected response field (expected {expected_field})"
                    )),
                };
            }

            DataSection {
                key: key.to_string(),
                title: title.to_string(),
                response_field: got_field,
                status: "ok".to_string(),
                ok: true,
                raw: format_response_payload(&response),
                error: None,
            }
        }
        Err(status) => {
            let (state, msg) = classify_status(&status);
            DataSection {
                key: key.to_string(),
                title: title.to_string(),
                response_field: expected_field.to_string(),
                status: state.to_string(),
                ok: state == "ok",
                raw: String::new(),
                error: Some(msg),
            }
        }
    }
}

pub fn idle_section(key: &str, title: &str, expected_field: &str) -> DataSection {
    DataSection {
        key: key.to_string(),
        title: title.to_string(),
        response_field: expected_field.to_string(),
        status: "idle".to_string(),
        ok: false,
        raw: String::new(),
        error: Some("Not loaded yet. Expand this section to fetch on demand.".to_string()),
    }
}

pub async fn call_handle(
    client: &mut DeviceClient<Channel>,
    request_variant: request::Request,
) -> Result<DeviceResponse, tonic::Status> {
    let request = DeviceRequest {
        id: 0,
        target_id: String::new(),
        epoch_id: 0,
        request: Some(request_variant),
    };

    client
        .handle(tonic::Request::new(request))
        .await
        .map(|resp| resp.into_inner())
}

pub fn format_response_payload(response: &DeviceResponse) -> String {
    let text = match response.response.as_ref() {
        Some(payload) => format!("{payload:#?}"),
        None => "<empty response payload>".to_string(),
    };

    truncate_for_ui(&text, 12000)
}

pub fn classify_status(status: &tonic::Status) -> (&'static str, String) {
    match status.code() {
        tonic::Code::Unimplemented => (
            "unsupported",
            "Endpoint is not implemented on this device's firmware/model.".to_string(),
        ),
        tonic::Code::PermissionDenied => (
            "restricted",
            "Endpoint is restricted by Starlink permissions.".to_string(),
        ),
        _ => ("error", format!("gRPC call failed: {status}")),
    }
}

pub fn truncate_for_ui(input: &str, max_chars: usize) -> String {
    if input.len() <= max_chars {
        return input.to_string();
    }

    let mut out = input[..max_chars].to_string();
    out.push_str("\n\n...output truncated for UI...");
    out
}

pub fn response_name(resp: &response::Response) -> &'static str {
    match resp {
        response::Response::Reboot(_) => "reboot",
        response::Response::Update(_) => "update",
        response::Response::GetLocation(_) => "get_location",
        response::Response::GetConnections(_) => "get_connections",
        response::Response::StartSpeedtest(_) => "start_speedtest",
        response::Response::GetSpeedtestStatus(_) => "get_speedtest_status",
        response::Response::DishStow(_) => "dish_stow",
        response::Response::DishGetContext(_) => "dish_get_context",
        response::Response::DishGetStatus(_) => "dish_get_status",
        response::Response::DishGetHistory(_) => "dish_get_history",
        response::Response::DishGetEmc(_) => "dish_get_emc",
        response::Response::DishGetConfig(_) => "dish_get_config",
        response::Response::DishInhibitGps(_) => "dish_inhibit_gps",
        response::Response::DishInhibitRf(_) => "dish_inhibit_rf",
        response::Response::DishGetDiagnostics(_) => "dish_get_diagnostics",
        response::Response::DishGetObstructionMap(_) => "dish_get_obstruction_map",
        response::Response::DishClearObstructionMap(_) => "dish_clear_obstruction_map",
        response::Response::GetGnssMeasurement(_) => "get_gnss_measurement",
        response::Response::WifiGetStatus(_) => "wifi_get_status",
        response::Response::WifiGetDiagnostics(_) => "wifi_get_diagnostics",
        response::Response::WifiGetConfig(_) => "wifi_get_config",
        response::Response::WifiGetHistory(_) => "wifi_get_history",
        response::Response::WifiGetPersistentStats(_) => "wifi_get_persistent_stats",
        response::Response::DishSetConfig(_) => "dish_set_config",
        response::Response::WifiSetConfig(_) => "wifi_set_config",
        _ => "other",
    }
}

pub fn normalize_address(input: Option<&str>, default: &str) -> String {
    let raw = input
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .trim_end_matches('/');

    if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    }
}
