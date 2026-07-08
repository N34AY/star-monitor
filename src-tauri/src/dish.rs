use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::Channel;

use crate::common::{
    call_handle, classify_status, connect_client, idle_section, normalize_address,
    response_name, run_section, DataSection,
};
use crate::pb::space_x::api::device::device_client::DeviceClient;
use crate::pb::space_x::api::device::request;
use crate::pb::space_x::api::device::response;
use crate::pb::space_x::api::device::{
    dish_config, dish_outage, DishAlerts, DishClearObstructionMapRequest, DishConfig,
    DishGetConfigRequest, DishGetConfigResponse, DishGetContextRequest, DishGetEmcRequest,
    DishGetObstructionMapRequest, DishGetStatusResponse, DishGpsStats, DishInhibitGpsRequest,
    DishInhibitRfRequest, DishOutage,
    DishSetConfigRequest, DishStowRequest, GetConnectionsRequest, GetDiagnosticsRequest,
    GetGnssMeasurementRequest, GetHistoryRequest, GetLocationRequest, GetLocationResponse,
    GetSpeedtestStatusRequest, GetStatusRequest, ObstructionMapReferenceFrame, PositionSource,
    RebootReason, RebootRequest, SoftwareUpdateState, StartSpeedtestRequest, UserMobilityClass,
};
use crate::pb::space_x::api::satellites::network::UtDisablementCode;

const DEFAULT_DISH_ADDRESS: &str = "http://192.168.100.1:9200";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DishAction {
    id: String,
    name: String,
    confirm: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMetrics {
    software_version: Option<String>,
    uptime_s: Option<u64>,
    downlink_mbps: Option<f64>,
    uplink_mbps: Option<f64>,
    latency_ms: Option<f64>,
    ping_drop_percent: Option<f64>,
    obstruction_percent: Option<f64>,
    gps_sats: Option<u32>,
    currently_obstructed: Option<bool>,
    boresight_azimuth_deg: Option<f64>,
    boresight_elevation_deg: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpsInfo {
    valid: bool,
    sats: u32,
    inhibited: bool,
    no_sats_after_ttff: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutageInfo {
    cause: String,
    duration_s: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObstructionDetail {
    fraction_obstructed_percent: f64,
    currently_obstructed: bool,
    time_obstructed_s: f64,
    avg_prolonged_obstruction_duration_s: Option<f64>,
    avg_prolonged_obstruction_interval_s: Option<f64>,
    patches_valid: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    lat: Option<f64>,
    lon: Option<f64>,
    alt_m: Option<f64>,
    source: String,
    accuracy_m: f64,
    horizontal_speed_mps: f64,
    vertical_speed_mps: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DishOverview {
    key_metrics: KeyMetrics,
    alerts: Vec<String>,
    outage: Option<OutageInfo>,
    disablement_code: String,
    gps: GpsInfo,
    obstruction: Option<ObstructionDetail>,
    mobility_class: String,
    software_update_state: String,
    stow_requested: bool,
    eth_speed_mbps: i32,
    connected_routers_count: usize,
    reboot_reason: String,
    last_boot_unix_s: u64,
    utc_offset_s: i32,
    country_code: String,
    location: Option<LocationInfo>,
    config: Option<DishConfigInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DishConfigInfo {
    snow_melt_mode: String,
    power_save_mode: bool,
    power_save_start_minutes: u32,
    power_save_duration_minutes: u32,
    level_dish_mode: String,
    swupdate_three_day_deferral_enabled: bool,
    swupdate_reboot_hour: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutageEvent {
    cause: String,
    start_unix_s: i64,
    duration_s: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DishHistory {
    latency_ms: Vec<f32>,
    ping_drop_percent: Vec<f32>,
    downlink_mbps: Vec<f32>,
    uplink_mbps: Vec<f32>,
    power_in_w: Vec<f32>,
    outages: Vec<OutageEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DishConfigPatch {
    snow_melt_mode: Option<String>,
    power_save_mode: Option<bool>,
    power_save_start_minutes: Option<u32>,
    power_save_duration_minutes: Option<u32>,
    level_dish_mode: Option<String>,
    swupdate_three_day_deferral_enabled: Option<bool>,
    swupdate_reboot_hour: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StarlinkSnapshot {
    dish_address: String,
    fetched_at_unix_s: u64,
    dish_up: bool,
    overview: Option<DishOverview>,
    sections: Vec<DataSection>,
    actions: Vec<DishAction>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObstructionMap {
    num_rows: u32,
    num_cols: u32,
    snr: Vec<f32>,
    max_theta_deg: f32,
    reference_frame: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecutionResult {
    id: String,
    ok: bool,
    message: String,
}

#[tauri::command]
pub fn default_dish_address() -> String {
    DEFAULT_DISH_ADDRESS.to_string()
}

#[tauri::command]
pub async fn fetch_starlink_snapshot(
    dish_address: Option<String>,
    include_location: Option<bool>,
    include_gnss: Option<bool>,
    include_history: Option<bool>,
) -> Result<StarlinkSnapshot, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;
    let mut sections = Vec::new();

    let (status_section, mut overview) = run_status_section(&mut client).await;
    let dish_up = status_section.ok;
    sections.push(status_section);

    sections.push(
        run_section(
            &mut client,
            "diagnostics",
            "Diagnostics",
            request::Request::GetDiagnostics(GetDiagnosticsRequest::default()),
            "dish_get_diagnostics",
        )
        .await,
    );

    if include_history.unwrap_or(false) {
        sections.push(
            run_section(
                &mut client,
                "history",
                "History",
                request::Request::GetHistory(GetHistoryRequest::default()),
                "dish_get_history",
            )
            .await,
        );
    } else {
        sections.push(idle_section("history", "History", "dish_get_history"));
    }

    sections.push(
        run_section(
            &mut client,
            "connections",
            "Connections",
            request::Request::GetConnections(GetConnectionsRequest::default()),
            "get_connections",
        )
        .await,
    );

    sections.push(
        run_section(
            &mut client,
            "speedtestStatus",
            "Speedtest Status",
            request::Request::GetSpeedtestStatus(GetSpeedtestStatusRequest::default()),
            "get_speedtest_status",
        )
        .await,
    );

    sections.push(
        run_section(
            &mut client,
            "dishContext",
            "Dish Context",
            request::Request::DishGetContext(DishGetContextRequest::default()),
            "dish_get_context",
        )
        .await,
    );

    let (dish_config_section, dish_config_info) = run_dish_config_section(&mut client).await;
    sections.push(dish_config_section);
    if let Some(ov) = overview.as_mut() {
        ov.config = dish_config_info;
    }

    sections.push(
        run_section(
            &mut client,
            "dishEmc",
            "Dish EMC",
            request::Request::DishGetEmc(DishGetEmcRequest::default()),
            "dish_get_emc",
        )
        .await,
    );

    if include_location.unwrap_or(true) {
        let (location_section, location) = run_location_section(&mut client).await;
        sections.push(location_section);
        if let Some(ov) = overview.as_mut() {
            ov.location = location;
        }
    }

    if include_gnss.unwrap_or(false) {
        sections.push(
            run_section(
                &mut client,
                "gnss",
                "GNSS Measurements",
                request::Request::GetGnssMeasurement(GetGnssMeasurementRequest::default()),
                "get_gnss_measurement",
            )
            .await,
        );
    }

    Ok(StarlinkSnapshot {
        dish_address: target,
        fetched_at_unix_s: now_unix_s(),
        dish_up,
        overview,
        sections,
        actions: available_actions(),
    })
}

#[tauri::command]
pub async fn fetch_starlink_section(
    dish_address: Option<String>,
    section_key: String,
) -> Result<DataSection, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;

    let section = match section_key.as_str() {
        "status" => run_status_section(&mut client).await.0,
        "diagnostics" => {
            run_section(
                &mut client,
                "diagnostics",
                "Diagnostics",
                request::Request::GetDiagnostics(GetDiagnosticsRequest::default()),
                "dish_get_diagnostics",
            )
            .await
        }
        "history" => {
            run_section(
                &mut client,
                "history",
                "History",
                request::Request::GetHistory(GetHistoryRequest::default()),
                "dish_get_history",
            )
            .await
        }
        "connections" => {
            run_section(
                &mut client,
                "connections",
                "Connections",
                request::Request::GetConnections(GetConnectionsRequest::default()),
                "get_connections",
            )
            .await
        }
        "speedtestStatus" => {
            run_section(
                &mut client,
                "speedtestStatus",
                "Speedtest Status",
                request::Request::GetSpeedtestStatus(GetSpeedtestStatusRequest::default()),
                "get_speedtest_status",
            )
            .await
        }
        "dishContext" => {
            run_section(
                &mut client,
                "dishContext",
                "Dish Context",
                request::Request::DishGetContext(DishGetContextRequest::default()),
                "dish_get_context",
            )
            .await
        }
        "dishConfig" => {
            run_section(
                &mut client,
                "dishConfig",
                "Dish Config",
                request::Request::DishGetConfig(DishGetConfigRequest::default()),
                "dish_get_config",
            )
            .await
        }
        "dishEmc" => {
            run_section(
                &mut client,
                "dishEmc",
                "Dish EMC",
                request::Request::DishGetEmc(DishGetEmcRequest::default()),
                "dish_get_emc",
            )
            .await
        }
        "location" => {
            run_section(
                &mut client,
                "location",
                "Location",
                request::Request::GetLocation(GetLocationRequest::default()),
                "get_location",
            )
            .await
        }
        "gnss" => {
            run_section(
                &mut client,
                "gnss",
                "GNSS Measurements",
                request::Request::GetGnssMeasurement(GetGnssMeasurementRequest::default()),
                "get_gnss_measurement",
            )
            .await
        }
        other => return Err(format!("unknown section key: {other}")),
    };

    Ok(section)
}

#[tauri::command]
pub async fn fetch_dish_obstruction_map(
    dish_address: Option<String>,
) -> Result<ObstructionMap, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;

    let response = call_handle(
        &mut client,
        request::Request::DishGetObstructionMap(DishGetObstructionMapRequest::default()),
    )
    .await
    .map_err(|s| format!("gRPC call failed: {s}"))?;

    match response.response {
        Some(response::Response::DishGetObstructionMap(map)) => Ok(ObstructionMap {
            num_rows: map.num_rows,
            num_cols: map.num_cols,
            snr: map.snr,
            max_theta_deg: map.max_theta_deg,
            reference_frame: ObstructionMapReferenceFrame::try_from(map.map_reference_frame)
                .unwrap_or(ObstructionMapReferenceFrame::FrameUnknown)
                .as_str_name()
                .to_string(),
        }),
        other => Err(format!(
            "unexpected response field for obstruction map: {:?}",
            other.map(|r| response_name(&r))
        )),
    }
}

const MAX_HISTORY_POINTS: usize = 300;

fn decimate(values: &[f32], max_points: usize) -> Vec<f32> {
    if max_points == 0 || values.len() <= max_points {
        return values.to_vec();
    }
    let stride = ((values.len() as f64) / (max_points as f64)).ceil() as usize;
    values.iter().step_by(stride.max(1)).copied().collect()
}

#[tauri::command]
pub async fn fetch_dish_history(dish_address: Option<String>) -> Result<DishHistory, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;

    let response = call_handle(
        &mut client,
        request::Request::GetHistory(GetHistoryRequest::default()),
    )
    .await
    .map_err(|s| format!("gRPC call failed: {s}"))?;

    match response.response {
        Some(response::Response::DishGetHistory(hist)) => {
            let downlink_mbps: Vec<f32> = hist
                .downlink_throughput_bps
                .iter()
                .map(|v| v / 1_000_000.0)
                .collect();
            let uplink_mbps: Vec<f32> = hist
                .uplink_throughput_bps
                .iter()
                .map(|v| v / 1_000_000.0)
                .collect();
            let ping_drop_percent: Vec<f32> =
                hist.pop_ping_drop_rate.iter().map(|v| v * 100.0).collect();

            let outages = hist
                .outages
                .iter()
                .map(|o| {
                    let cause = dish_outage::Cause::try_from(o.cause)
                        .unwrap_or(dish_outage::Cause::Unknown)
                        .as_str_name()
                        .to_string();
                    OutageEvent {
                        cause,
                        start_unix_s: o.start_timestamp_ns / 1_000_000_000,
                        duration_s: (o.duration_ns as f64) / 1_000_000_000.0,
                    }
                })
                .collect();

            Ok(DishHistory {
                latency_ms: decimate(&hist.pop_ping_latency_ms, MAX_HISTORY_POINTS),
                ping_drop_percent: decimate(&ping_drop_percent, MAX_HISTORY_POINTS),
                downlink_mbps: decimate(&downlink_mbps, MAX_HISTORY_POINTS),
                uplink_mbps: decimate(&uplink_mbps, MAX_HISTORY_POINTS),
                power_in_w: decimate(&hist.power_in, MAX_HISTORY_POINTS),
                outages,
            })
        }
        other => Err(format!(
            "unexpected response field for history: {:?}",
            other.map(|r| response_name(&r))
        )),
    }
}

#[tauri::command]
pub async fn apply_dish_config(
    dish_address: Option<String>,
    patch: DishConfigPatch,
) -> Result<ActionExecutionResult, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;

    let mut cfg = DishConfig::default();

    if let Some(mode) = &patch.snow_melt_mode {
        cfg.snow_melt_mode = dish_config::SnowMeltMode::from_str_name(mode)
            .ok_or_else(|| format!("unknown snowMeltMode: {mode}"))? as i32;
        cfg.apply_snow_melt_mode = true;
    }
    if let Some(v) = patch.power_save_mode {
        cfg.power_save_mode = v;
        cfg.apply_power_save_mode = true;
    }
    if let Some(v) = patch.power_save_start_minutes {
        cfg.power_save_start_minutes = v;
        cfg.apply_power_save_start_minutes = true;
    }
    if let Some(v) = patch.power_save_duration_minutes {
        cfg.power_save_duration_minutes = v;
        cfg.apply_power_save_duration_minutes = true;
    }
    if let Some(mode) = &patch.level_dish_mode {
        cfg.level_dish_mode = dish_config::LevelDishMode::from_str_name(mode)
            .ok_or_else(|| format!("unknown levelDishMode: {mode}"))? as i32;
        cfg.apply_level_dish_mode = true;
    }
    if let Some(v) = patch.swupdate_three_day_deferral_enabled {
        cfg.swupdate_three_day_deferral_enabled = v;
        cfg.apply_swupdate_three_day_deferral_enabled = true;
    }
    if let Some(v) = patch.swupdate_reboot_hour {
        cfg.swupdate_reboot_hour = v;
        cfg.apply_swupdate_reboot_hour = true;
    }

    let response = call_handle(
        &mut client,
        request::Request::DishSetConfig(DishSetConfigRequest {
            dish_config: Some(cfg),
        }),
    )
    .await
    .map_err(|s| format!("gRPC call failed: {s}"))?;

    let got = response
        .response
        .as_ref()
        .map(response_name)
        .unwrap_or("none");
    if got != "dish_set_config" {
        return Err(format!(
            "unexpected response field for apply_dish_config: got {got}"
        ));
    }

    Ok(ActionExecutionResult {
        id: "applyDishConfig".to_string(),
        ok: true,
        message: "Dish configuration updated".to_string(),
    })
}

#[tauri::command]
pub async fn trigger_dish_action(
    dish_address: Option<String>,
    action_id: String,
) -> Result<ActionExecutionResult, String> {
    let target = normalize_address(dish_address.as_deref(), DEFAULT_DISH_ADDRESS);
    let mut client = connect_client(&target).await?;

    let (request_variant, expected, message) = match action_id.as_str() {
        "reboot" => (
            request::Request::Reboot(RebootRequest::default()),
            "reboot",
            "Dish reboot request sent",
        ),
        "stow" => (
            request::Request::DishStow(DishStowRequest { unstow: false }),
            "dish_stow",
            "Dish stow request sent",
        ),
        "unstow" => (
            request::Request::DishStow(DishStowRequest { unstow: true }),
            "dish_stow",
            "Dish unstow request sent",
        ),
        "startSpeedtest" => (
            request::Request::StartSpeedtest(StartSpeedtestRequest::default()),
            "start_speedtest",
            "Speedtest start request sent",
        ),
        "gpsInhibitOn" => (
            request::Request::DishInhibitGps(DishInhibitGpsRequest { inhibit_gps: true }),
            "dish_inhibit_gps",
            "GPS inhibit enabled",
        ),
        "gpsInhibitOff" => (
            request::Request::DishInhibitGps(DishInhibitGpsRequest { inhibit_gps: false }),
            "dish_inhibit_gps",
            "GPS inhibit disabled",
        ),
        "rfInhibitOn" => (
            request::Request::DishInhibitRf(DishInhibitRfRequest { inhibit_rf: true }),
            "dish_inhibit_rf",
            "RF inhibit enabled",
        ),
        "rfInhibitOff" => (
            request::Request::DishInhibitRf(DishInhibitRfRequest { inhibit_rf: false }),
            "dish_inhibit_rf",
            "RF inhibit disabled",
        ),
        "clearObstructionMap" => (
            request::Request::DishClearObstructionMap(DishClearObstructionMapRequest::default()),
            "dish_clear_obstruction_map",
            "Obstruction map cleared",
        ),
        other => {
            return Err(format!("unknown action id: {other}"));
        }
    };

    let response = match call_handle(&mut client, request_variant).await {
        Ok(resp) => resp,
        // A reboot makes the dish drop the connection as it restarts, often
        // before the response finishes - that's the connection tearing down
        // as *expected*, not a failed request, since we already connected
        // successfully moments earlier.
        Err(_status) if action_id == "reboot" => {
            return Ok(ActionExecutionResult {
                id: action_id,
                ok: true,
                message: message.to_string(),
            });
        }
        Err(status) => return Err(format!("gRPC call failed: {status}")),
    };
    let got = response
        .response
        .as_ref()
        .map(response_name)
        .unwrap_or("none");
    if got != expected {
        return Err(format!(
            "unexpected response field for action {action_id}: got {got}, expected {expected}"
        ));
    }

    Ok(ActionExecutionResult {
        id: action_id,
        ok: true,
        message: message.to_string(),
    })
}

async fn run_status_section(
    client: &mut DeviceClient<Channel>,
) -> (DataSection, Option<DishOverview>) {
    let status_call = call_handle(client, request::Request::GetStatus(GetStatusRequest::default())).await;
    match status_call {
        Ok(resp) => {
            let got_field = resp
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != "dish_get_status" {
                return (
                    DataSection {
                        key: "status".to_string(),
                        title: "Dish Status".to_string(),
                        response_field: got_field,
                        status: "error".to_string(),
                        ok: false,
                        raw: String::new(),
                        error: Some("unexpected response field (expected dish_get_status)".to_string()),
                    },
                    None,
                );
            }

            let overview = match resp.response.as_ref() {
                Some(response::Response::DishGetStatus(payload)) => Some(build_overview(payload)),
                _ => None,
            };

            (
                DataSection {
                    key: "status".to_string(),
                    title: "Dish Status".to_string(),
                    response_field: got_field,
                    status: "ok".to_string(),
                    ok: true,
                    raw: crate::common::format_response_payload(&resp),
                    error: None,
                },
                overview,
            )
        }
        Err(status) => {
            let (state, msg) = classify_status(&status);
            (
                DataSection {
                    key: "status".to_string(),
                    title: "Dish Status".to_string(),
                    response_field: "dish_get_status".to_string(),
                    status: state.to_string(),
                    ok: state == "ok",
                    raw: String::new(),
                    error: Some(msg),
                },
                None,
            )
        }
    }
}

async fn run_location_section(
    client: &mut DeviceClient<Channel>,
) -> (DataSection, Option<LocationInfo>) {
    match call_handle(client, request::Request::GetLocation(GetLocationRequest::default())).await {
        Ok(resp) => {
            let got_field = resp
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != "get_location" {
                return (
                    DataSection {
                        key: "location".to_string(),
                        title: "Location".to_string(),
                        response_field: got_field,
                        status: "error".to_string(),
                        ok: false,
                        raw: String::new(),
                        error: Some("unexpected response field (expected get_location)".to_string()),
                    },
                    None,
                );
            }

            let location = match resp.response.as_ref() {
                Some(response::Response::GetLocation(payload)) => Some(build_location_info(payload)),
                _ => None,
            };

            (
                DataSection {
                    key: "location".to_string(),
                    title: "Location".to_string(),
                    response_field: got_field,
                    status: "ok".to_string(),
                    ok: true,
                    raw: crate::common::format_response_payload(&resp),
                    error: None,
                },
                location,
            )
        }
        Err(status) => {
            let (state, msg) = classify_status(&status);
            (
                DataSection {
                    key: "location".to_string(),
                    title: "Location".to_string(),
                    response_field: "get_location".to_string(),
                    status: state.to_string(),
                    ok: state == "ok",
                    raw: String::new(),
                    error: Some(msg),
                },
                None,
            )
        }
    }
}

fn build_location_info(loc: &GetLocationResponse) -> LocationInfo {
    let lla = loc.lla.as_ref();
    LocationInfo {
        lat: lla.map(|l| l.lat),
        lon: lla.map(|l| l.lon),
        alt_m: lla.map(|l| l.alt),
        source: PositionSource::try_from(loc.source)
            .map(|s| s.as_str_name().to_string())
            .unwrap_or_else(|_| "UNKNOWN".to_string()),
        accuracy_m: loc.sigma_m,
        horizontal_speed_mps: loc.horizontal_speed_mps,
        vertical_speed_mps: loc.vertical_speed_mps,
    }
}

async fn run_dish_config_section(
    client: &mut DeviceClient<Channel>,
) -> (DataSection, Option<DishConfigInfo>) {
    match call_handle(client, request::Request::DishGetConfig(DishGetConfigRequest::default())).await {
        Ok(resp) => {
            let got_field = resp
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != "dish_get_config" {
                return (
                    DataSection {
                        key: "dishConfig".to_string(),
                        title: "Dish Config".to_string(),
                        response_field: got_field,
                        status: "error".to_string(),
                        ok: false,
                        raw: String::new(),
                        error: Some("unexpected response field (expected dish_get_config)".to_string()),
                    },
                    None,
                );
            }

            let config = match resp.response.as_ref() {
                Some(response::Response::DishGetConfig(payload)) => build_dish_config_info(payload),
                _ => None,
            };

            (
                DataSection {
                    key: "dishConfig".to_string(),
                    title: "Dish Config".to_string(),
                    response_field: got_field,
                    status: "ok".to_string(),
                    ok: true,
                    raw: crate::common::format_response_payload(&resp),
                    error: None,
                },
                config,
            )
        }
        Err(status) => {
            let (state, msg) = classify_status(&status);
            (
                DataSection {
                    key: "dishConfig".to_string(),
                    title: "Dish Config".to_string(),
                    response_field: "dish_get_config".to_string(),
                    status: state.to_string(),
                    ok: state == "ok",
                    raw: String::new(),
                    error: Some(msg),
                },
                None,
            )
        }
    }
}

fn build_dish_config_info(resp: &DishGetConfigResponse) -> Option<DishConfigInfo> {
    let cfg = resp.dish_config.as_ref()?;
    Some(DishConfigInfo {
        snow_melt_mode: dish_config::SnowMeltMode::try_from(cfg.snow_melt_mode)
            .unwrap_or(dish_config::SnowMeltMode::Auto)
            .as_str_name()
            .to_string(),
        power_save_mode: cfg.power_save_mode,
        power_save_start_minutes: cfg.power_save_start_minutes,
        power_save_duration_minutes: cfg.power_save_duration_minutes,
        level_dish_mode: dish_config::LevelDishMode::try_from(cfg.level_dish_mode)
            .unwrap_or(dish_config::LevelDishMode::TiltLikeNormal)
            .as_str_name()
            .to_string(),
        swupdate_three_day_deferral_enabled: cfg.swupdate_three_day_deferral_enabled,
        swupdate_reboot_hour: cfg.swupdate_reboot_hour,
    })
}

fn build_overview(status: &DishGetStatusResponse) -> DishOverview {
    let software_version = status
        .device_info
        .as_ref()
        .map(|i| i.software_version.clone())
        .filter(|s| !s.is_empty());

    let uptime_s = status.device_state.as_ref().map(|s| s.uptime_s);
    let downlink_mbps = Some((status.downlink_throughput_bps as f64) / 1_000_000.0);
    let uplink_mbps = Some((status.uplink_throughput_bps as f64) / 1_000_000.0);
    let latency_ms = Some(status.pop_ping_latency_ms as f64);
    let ping_drop_percent = Some((status.pop_ping_drop_rate as f64) * 100.0);

    let obstruction_percent = status
        .obstruction_stats
        .as_ref()
        .map(|o| (o.fraction_obstructed as f64) * 100.0);
    let gps_sats = status.gps_stats.as_ref().map(|g| g.gps_sats);
    let currently_obstructed = status
        .obstruction_stats
        .as_ref()
        .map(|o| o.currently_obstructed);

    let key_metrics = KeyMetrics {
        software_version,
        uptime_s,
        downlink_mbps,
        uplink_mbps,
        latency_ms,
        ping_drop_percent,
        obstruction_percent,
        gps_sats,
        currently_obstructed,
        boresight_azimuth_deg: Some(status.boresight_azimuth_deg as f64),
        boresight_elevation_deg: Some(status.boresight_elevation_deg as f64),
    };

    let alerts = status
        .alerts
        .as_ref()
        .map(dish_alert_labels)
        .unwrap_or_default();

    let outage = status.outage.as_ref().and_then(build_outage_info);

    let disablement_code = UtDisablementCode::try_from(status.disablement_code)
        .map(|c| c.as_str_name().to_string())
        .unwrap_or_else(|_| "UNKNOWN".to_string());

    let gps = status
        .gps_stats
        .as_ref()
        .map(|g: &DishGpsStats| GpsInfo {
            valid: g.gps_valid,
            sats: g.gps_sats,
            inhibited: g.inhibit_gps,
            no_sats_after_ttff: g.no_sats_after_ttff,
        })
        .unwrap_or(GpsInfo {
            valid: false,
            sats: 0,
            inhibited: false,
            no_sats_after_ttff: false,
        });

    let obstruction = status.obstruction_stats.as_ref().map(|o| ObstructionDetail {
        fraction_obstructed_percent: (o.fraction_obstructed as f64) * 100.0,
        currently_obstructed: o.currently_obstructed,
        time_obstructed_s: o.time_obstructed as f64,
        avg_prolonged_obstruction_duration_s: o
            .avg_prolonged_obstruction_valid
            .then_some(o.avg_prolonged_obstruction_duration_s as f64),
        avg_prolonged_obstruction_interval_s: o
            .avg_prolonged_obstruction_valid
            .then_some(o.avg_prolonged_obstruction_interval_s as f64),
        patches_valid: o.patches_valid,
    });

    let mobility_class = UserMobilityClass::try_from(status.mobility_class)
        .unwrap_or(UserMobilityClass::Stationary)
        .as_str_name()
        .to_string();
    let software_update_state = SoftwareUpdateState::try_from(status.software_update_state)
        .unwrap_or(SoftwareUpdateState::Unknown)
        .as_str_name()
        .to_string();

    let reboot_reason = RebootReason::try_from(status.reboot_reason)
        .unwrap_or(RebootReason::None)
        .as_str_name()
        .to_string();
    let uptime_for_boot = status.device_state.as_ref().map(|s| s.uptime_s).unwrap_or(0);
    let last_boot_unix_s = now_unix_s().saturating_sub(uptime_for_boot);
    let utc_offset_s = status.device_info.as_ref().map(|i| i.utc_offset_s).unwrap_or(0);
    let country_code = status
        .device_info
        .as_ref()
        .map(|i| i.country_code.clone())
        .unwrap_or_default();

    DishOverview {
        key_metrics,
        alerts,
        outage,
        disablement_code,
        gps,
        obstruction,
        mobility_class,
        software_update_state,
        stow_requested: status.stow_requested,
        eth_speed_mbps: status.eth_speed_mbps,
        connected_routers_count: status.connected_routers.len(),
        reboot_reason,
        last_boot_unix_s,
        utc_offset_s,
        country_code,
        location: None,
        config: None,
    }
}

fn build_outage_info(outage: &DishOutage) -> Option<OutageInfo> {
    use crate::pb::space_x::api::device::dish_outage::Cause;

    let cause = Cause::try_from(outage.cause).unwrap_or(Cause::Unknown);
    if cause == Cause::Unknown && outage.duration_ns == 0 {
        return None;
    }

    Some(OutageInfo {
        cause: cause.as_str_name().to_string(),
        duration_s: (outage.duration_ns as f64) / 1_000_000_000.0,
    })
}

fn dish_alert_labels(alerts: &DishAlerts) -> Vec<String> {
    let mut labels = Vec::new();
    macro_rules! push_if {
        ($field:ident, $label:expr) => {
            if alerts.$field {
                labels.push($label.to_string());
            }
        };
    }

    push_if!(motors_stuck, "Motors stuck");
    push_if!(thermal_shutdown, "Thermal shutdown");
    push_if!(thermal_throttle, "Thermal throttle");
    push_if!(unexpected_location, "Unexpected location");
    push_if!(mast_not_near_vertical, "Mast not near vertical");
    push_if!(slow_ethernet_speeds, "Slow ethernet speeds");
    push_if!(roaming, "Roaming");
    push_if!(install_pending, "Install pending");
    push_if!(is_heating, "Heating");
    push_if!(power_supply_thermal_throttle, "Power supply thermal throttle");
    push_if!(dbf_telem_stale, "Telemetry stale");
    push_if!(low_motor_current, "Low motor current");
    push_if!(lower_signal_than_predicted, "Lower signal than predicted");
    push_if!(slow_ethernet_speeds_100, "Slow ethernet speeds (100M)");
    push_if!(dish_water_detected, "Water detected on dish");
    push_if!(router_water_detected, "Water detected on router");
    push_if!(upsu_router_port_slow, "UPSU router port slow");
    push_if!(no_ethernet_link, "No ethernet link");

    labels
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn available_actions() -> Vec<DishAction> {
    vec![
        DishAction {
            id: "reboot".to_string(),
            name: "Reboot Dish".to_string(),
            confirm: Some("Reboot Starlink dish now? This causes temporary outage.".to_string()),
        },
        DishAction {
            id: "stow".to_string(),
            name: "Stow Dish".to_string(),
            confirm: Some("Stow the dish now?".to_string()),
        },
        DishAction {
            id: "unstow".to_string(),
            name: "Unstow Dish".to_string(),
            confirm: None,
        },
        DishAction {
            id: "startSpeedtest".to_string(),
            name: "Start Speedtest".to_string(),
            confirm: None,
        },
        DishAction {
            id: "gpsInhibitOn".to_string(),
            name: "Enable GPS Inhibit".to_string(),
            confirm: None,
        },
        DishAction {
            id: "gpsInhibitOff".to_string(),
            name: "Disable GPS Inhibit".to_string(),
            confirm: None,
        },
        DishAction {
            id: "rfInhibitOn".to_string(),
            name: "Enable RF Inhibit".to_string(),
            confirm: Some("Disable RF transmissions now?".to_string()),
        },
        DishAction {
            id: "rfInhibitOff".to_string(),
            name: "Disable RF Inhibit".to_string(),
            confirm: None,
        },
        DishAction {
            id: "clearObstructionMap".to_string(),
            name: "Clear Obstruction Map".to_string(),
            confirm: Some("Clear the dish's learned obstruction map?".to_string()),
        },
    ]
}
