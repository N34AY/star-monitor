use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::Channel;

use crate::common::{
    call_handle, classify_status, connect_client, normalize_address, response_name, run_section,
    DataSection,
};
use crate::pb::space_x::api::device::device_client::DeviceClient;
use crate::pb::space_x::api::device::request;
use crate::pb::space_x::api::device::response;
use crate::pb::space_x::api::device::{
    wifi_client, wifi_config, AuthWpa2, AuthWpa2Wpa3, GetDiagnosticsRequest,
    GetHistoryRequest, GetPersistentStatsRequest, GetStatusRequest, RebootRequest, UpdateRequest,
    WifiAlerts, WifiClient, WifiConfig, WifiGetConfigRequest, WifiGetStatusResponse,
    WifiSetConfigRequest,
};

const DEFAULT_ROUTER_ADDRESS: &str = "http://192.168.1.1:9000";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterAction {
    id: String,
    name: String,
    confirm: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiClientInfo {
    name: String,
    mac_address: String,
    ip_address: String,
    signal_strength: f64,
    snr: f64,
    associated_time_s: u32,
    iface: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterOverview {
    software_version: Option<String>,
    pub(crate) uptime_s: Option<u64>,
    pub(crate) ping_latency_ms: f64,
    pub(crate) ping_drop_percent: f64,
    dish_ping_latency_ms: f64,
    dish_ping_drop_percent: f64,
    pub(crate) alerts: Vec<String>,
    clients: Vec<WifiClientInfo>,
    ssids: Vec<String>,
    setup_complete: bool,
    bypass_mode: bool,
    pub(crate) reboot_from_update: Option<RebootEventInfo>,
    config: Option<WifiConfigInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetworkInfo {
    band: String,
    ssid: String,
    hidden: bool,
    disabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiConfigInfo {
    channel_2ghz: u32,
    channel_5ghz: u32,
    dfs_enabled: bool,
    secure_dns: bool,
    bypass_mode: bool,
    networks: Vec<WifiNetworkInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebootEventInfo {
    pub(crate) count: u32,
    pub(crate) last_occurred_unix_s: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterHistory {
    ping_latency_ms: Vec<f32>,
    ping_drop_percent: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPatch {
    band: String,
    ssid: Option<String>,
    password: Option<String>,
    hidden: Option<bool>,
    disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiConfigPatch {
    channel_2ghz: Option<u32>,
    channel_5ghz: Option<u32>,
    dfs_enabled: Option<bool>,
    secure_dns: Option<bool>,
    bypass_mode: Option<bool>,
    network: Option<NetworkPatch>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterSnapshot {
    router_address: String,
    fetched_at_unix_s: u64,
    router_up: bool,
    overview: Option<RouterOverview>,
    sections: Vec<DataSection>,
    actions: Vec<RouterAction>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterActionExecutionResult {
    id: String,
    ok: bool,
    message: String,
}

#[tauri::command]
pub fn default_router_address() -> String {
    DEFAULT_ROUTER_ADDRESS.to_string()
}

/// Result of a single lightweight status probe (just `GetStatus`) - used by
/// the background history collector instead of the full snapshot's endpoint
/// list, to keep per-tick load on the router minimal.
pub struct RouterProbe {
    pub reachable: bool,
    pub overview: Option<RouterOverview>,
    pub error: Option<String>,
}

pub async fn probe_router_status(router_address: &str) -> RouterProbe {
    match connect_client(router_address).await {
        Ok(mut client) => {
            let (section, overview) = run_status_section(&mut client).await;
            RouterProbe {
                reachable: section.ok,
                overview,
                error: section.error,
            }
        }
        Err(e) => RouterProbe {
            reachable: false,
            overview: None,
            error: Some(e),
        },
    }
}

#[tauri::command]
pub async fn fetch_router_snapshot(router_address: Option<String>) -> Result<RouterSnapshot, String> {
    let target = normalize_address(router_address.as_deref(), DEFAULT_ROUTER_ADDRESS);
    let mut client = connect_client(&target).await?;
    let mut sections = Vec::new();

    let (status_section, mut overview) = run_status_section(&mut client).await;
    let router_up = status_section.ok;
    sections.push(status_section);

    sections.push(
        run_section(
            &mut client,
            "diagnostics",
            "Diagnostics",
            request::Request::GetDiagnostics(GetDiagnosticsRequest::default()),
            "wifi_get_diagnostics",
        )
        .await,
    );

    let (config_section, config_info) = run_wifi_config_section(&mut client).await;
    sections.push(config_section);

    if let Some(ov) = overview.as_mut() {
        ov.reboot_from_update = fetch_reboot_from_update(&mut client).await;
        ov.config = config_info;
    }

    Ok(RouterSnapshot {
        router_address: target,
        fetched_at_unix_s: now_unix_s(),
        router_up,
        overview,
        sections,
        actions: available_actions(),
    })
}

#[tauri::command]
pub async fn fetch_router_section(
    router_address: Option<String>,
    section_key: String,
) -> Result<DataSection, String> {
    let target = normalize_address(router_address.as_deref(), DEFAULT_ROUTER_ADDRESS);
    let mut client = connect_client(&target).await?;

    let section = match section_key.as_str() {
        "status" => run_status_section(&mut client).await.0,
        "diagnostics" => {
            run_section(
                &mut client,
                "diagnostics",
                "Diagnostics",
                request::Request::GetDiagnostics(GetDiagnosticsRequest::default()),
                "wifi_get_diagnostics",
            )
            .await
        }
        "config" => {
            run_section(
                &mut client,
                "config",
                "WiFi Config",
                request::Request::WifiGetConfig(WifiGetConfigRequest::default()),
                "wifi_get_config",
            )
            .await
        }
        other => return Err(format!("unknown section key: {other}")),
    };

    Ok(section)
}

#[tauri::command]
pub async fn trigger_router_action(
    router_address: Option<String>,
    action_id: String,
) -> Result<RouterActionExecutionResult, String> {
    let target = normalize_address(router_address.as_deref(), DEFAULT_ROUTER_ADDRESS);
    let mut client = connect_client(&target).await?;

    let (request_variant, expected, message) = match action_id.as_str() {
        "reboot" => (
            request::Request::Reboot(RebootRequest::default()),
            "reboot",
            "Router reboot request sent",
        ),
        "checkForUpdate" => (
            request::Request::Update(UpdateRequest::default()),
            "update",
            "Update check request sent",
        ),
        other => {
            return Err(format!("unknown action id: {other}"));
        }
    };

    let response = match call_handle(&mut client, request_variant).await {
        Ok(resp) => resp,
        // A reboot makes the router drop the connection as it restarts, often
        // before the response finishes - that's the connection tearing down
        // as *expected*, not a failed request, since we already connected
        // successfully moments earlier.
        Err(_status) if action_id == "reboot" => {
            return Ok(RouterActionExecutionResult {
                id: action_id,
                ok: true,
                message: message.to_string(),
            });
        }
        Err(status) => return Err(format!("gRPC call failed: {status}")),
    };
    let got = response.response.as_ref().map(response_name).unwrap_or("none");
    if got != expected {
        return Err(format!(
            "unexpected response field for action {action_id}: got {got}, expected {expected}"
        ));
    }

    Ok(RouterActionExecutionResult {
        id: action_id,
        ok: true,
        message: message.to_string(),
    })
}

const MAX_HISTORY_POINTS: usize = 300;

fn decimate(values: &[f32], max_points: usize) -> Vec<f32> {
    if max_points == 0 || values.len() <= max_points {
        return values.to_vec();
    }
    let stride = ((values.len() as f64) / (max_points as f64)).ceil() as usize;
    values.iter().step_by(stride.max(1)).copied().collect()
}

async fn fetch_reboot_from_update(client: &mut DeviceClient<Channel>) -> Option<RebootEventInfo> {
    let response = call_handle(
        client,
        request::Request::GetPersistentStats(GetPersistentStatsRequest::default()),
    )
    .await
    .ok()?;

    match response.response {
        Some(response::Response::WifiGetPersistentStats(payload)) => {
            let event = payload.stats?.reboot_from_software_update?;
            Some(RebootEventInfo {
                count: event.count,
                last_occurred_unix_s: event.last_occurred_timestamp,
            })
        }
        _ => None,
    }
}

#[tauri::command]
pub async fn fetch_router_history(router_address: Option<String>) -> Result<RouterHistory, String> {
    let target = normalize_address(router_address.as_deref(), DEFAULT_ROUTER_ADDRESS);
    let mut client = connect_client(&target).await?;

    let response = call_handle(
        &mut client,
        request::Request::GetHistory(GetHistoryRequest::default()),
    )
    .await
    .map_err(|s| format!("gRPC call failed: {s}"))?;

    match response.response {
        Some(response::Response::WifiGetHistory(hist)) => {
            let ping_drop_percent: Vec<f32> =
                hist.ping_drop_rate.iter().map(|v| v * 100.0).collect();
            Ok(RouterHistory {
                ping_latency_ms: decimate(&hist.ping_latency_ms, MAX_HISTORY_POINTS),
                ping_drop_percent: decimate(&ping_drop_percent, MAX_HISTORY_POINTS),
            })
        }
        other => Err(format!(
            "unexpected response field for router history: {:?}",
            other.map(|r| response_name(&r))
        )),
    }
}

#[tauri::command]
pub async fn apply_wifi_config(
    router_address: Option<String>,
    patch: WifiConfigPatch,
) -> Result<RouterActionExecutionResult, String> {
    let target = normalize_address(router_address.as_deref(), DEFAULT_ROUTER_ADDRESS);
    let mut client = connect_client(&target).await?;

    let mut cfg = WifiConfig::default();

    if let Some(v) = patch.channel_2ghz {
        cfg.channel_2ghz = v;
        cfg.apply_channel_2ghz = true;
    }
    if let Some(v) = patch.channel_5ghz {
        cfg.channel_5ghz = v;
        cfg.apply_channel_5ghz = true;
    }
    if let Some(v) = patch.dfs_enabled {
        cfg.dfs_enabled = v;
        cfg.apply_dfs_enabled = true;
    }
    if let Some(v) = patch.secure_dns {
        cfg.secure_dns = v;
        cfg.apply_secure_dns = true;
    }
    if let Some(v) = patch.bypass_mode {
        cfg.bypass_mode = v;
        cfg.apply_bypass_mode = true;
    }

    if let Some(net_patch) = &patch.network {
        let current = call_handle(
            &mut client,
            request::Request::WifiGetConfig(WifiGetConfigRequest::default()),
        )
        .await
        .map_err(|s| format!("gRPC call failed: {s}"))?;

        let mut networks = match current.response {
            Some(response::Response::WifiGetConfig(resp)) => {
                resp.wifi_config.map(|c| c.networks).unwrap_or_default()
            }
            _ => return Err("failed to read current WiFi config".to_string()),
        };

        let target_band = wifi_config::Band::from_str_name(&net_patch.band)
            .ok_or_else(|| format!("unknown band: {}", net_patch.band))?;

        let mut applied = false;
        for network in networks.iter_mut() {
            for bss in network.basic_service_sets.iter_mut() {
                if bss.band != target_band as i32 {
                    continue;
                }
                if let Some(ssid) = &net_patch.ssid {
                    bss.ssid = ssid.clone();
                }
                if let Some(hidden) = net_patch.hidden {
                    bss.hidden = hidden;
                }
                if let Some(disabled) = net_patch.disabled {
                    bss.disable = disabled;
                }
                if let Some(password) = &net_patch.password {
                    bss.auth = Some(match bss.auth.take() {
                        Some(wifi_config::basic_service_set::Auth::AuthWpa2Wpa3(_)) => {
                            wifi_config::basic_service_set::Auth::AuthWpa2Wpa3(AuthWpa2Wpa3 {
                                password: password.clone(),
                            })
                        }
                        _ => wifi_config::basic_service_set::Auth::AuthWpa2(AuthWpa2 {
                            password: password.clone(),
                        }),
                    });
                }
                applied = true;
            }
        }

        if !applied {
            return Err(format!("no configured network found for band {}", net_patch.band));
        }

        cfg.networks = networks;
        cfg.apply_networks = true;
    }

    let response = call_handle(
        &mut client,
        request::Request::WifiSetConfig(WifiSetConfigRequest {
            wifi_config: Some(cfg),
        }),
    )
    .await
    .map_err(|s| format!("gRPC call failed: {s}"))?;

    let got = response
        .response
        .as_ref()
        .map(response_name)
        .unwrap_or("none");
    if got != "wifi_set_config" {
        return Err(format!(
            "unexpected response field for apply_wifi_config: got {got}"
        ));
    }

    Ok(RouterActionExecutionResult {
        id: "applyWifiConfig".to_string(),
        ok: true,
        message: "Router configuration updated".to_string(),
    })
}

async fn run_status_section(client: &mut DeviceClient<Channel>) -> (DataSection, Option<RouterOverview>) {
    let status_call = call_handle(client, request::Request::GetStatus(GetStatusRequest::default())).await;
    match status_call {
        Ok(resp) => {
            let got_field = resp
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != "wifi_get_status" {
                return (
                    DataSection {
                        key: "status".to_string(),
                        title: "Router Status".to_string(),
                        response_field: got_field,
                        status: "error".to_string(),
                        ok: false,
                        raw: String::new(),
                        error: Some("unexpected response field (expected wifi_get_status)".to_string()),
                    },
                    None,
                );
            }

            let overview = match resp.response.as_ref() {
                Some(response::Response::WifiGetStatus(payload)) => Some(build_overview(payload)),
                _ => None,
            };

            (
                DataSection {
                    key: "status".to_string(),
                    title: "Router Status".to_string(),
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
                    title: "Router Status".to_string(),
                    response_field: "wifi_get_status".to_string(),
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

fn build_overview(status: &WifiGetStatusResponse) -> RouterOverview {
    let software_version = status
        .device_info
        .as_ref()
        .map(|i| i.software_version.clone())
        .filter(|s| !s.is_empty());
    let uptime_s = status.device_state.as_ref().map(|s| s.uptime_s);

    let alerts = status.alerts.as_ref().map(wifi_alert_labels).unwrap_or_default();

    let mut clients: Vec<WifiClientInfo> = status.clients.iter().map(client_info).collect();
    clients.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut ssids: Vec<String> = status
        .config
        .as_ref()
        .map(|c| {
            c.networks
                .iter()
                .flat_map(|n| n.basic_service_sets.iter())
                .filter(|b| !b.hidden && !b.disable)
                .map(|b| b.ssid.clone())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ssids.sort();
    ssids.dedup();

    RouterOverview {
        software_version,
        uptime_s,
        ping_latency_ms: status.ping_latency_ms as f64,
        ping_drop_percent: (status.ping_drop_rate as f64) * 100.0,
        dish_ping_latency_ms: status.dish_ping_latency_ms as f64,
        dish_ping_drop_percent: (status.dish_ping_drop_rate as f64) * 100.0,
        alerts,
        clients,
        ssids,
        setup_complete: status.config.as_ref().map(|c| c.setup_complete).unwrap_or(false),
        bypass_mode: status.config.as_ref().map(|c| c.bypass_mode).unwrap_or(false),
        reboot_from_update: None,
        config: None,
    }
}

async fn run_wifi_config_section(
    client: &mut DeviceClient<Channel>,
) -> (DataSection, Option<WifiConfigInfo>) {
    match call_handle(
        client,
        request::Request::WifiGetConfig(WifiGetConfigRequest::default()),
    )
    .await
    {
        Ok(resp) => {
            let got_field = resp
                .response
                .as_ref()
                .map(response_name)
                .unwrap_or("none")
                .to_string();

            if got_field != "wifi_get_config" {
                return (
                    DataSection {
                        key: "config".to_string(),
                        title: "WiFi Config".to_string(),
                        response_field: got_field,
                        status: "error".to_string(),
                        ok: false,
                        raw: String::new(),
                        error: Some("unexpected response field (expected wifi_get_config)".to_string()),
                    },
                    None,
                );
            }

            let config = match resp.response.as_ref() {
                Some(response::Response::WifiGetConfig(payload)) => build_wifi_config_info(payload),
                _ => None,
            };

            (
                DataSection {
                    key: "config".to_string(),
                    title: "WiFi Config".to_string(),
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
                    key: "config".to_string(),
                    title: "WiFi Config".to_string(),
                    response_field: "wifi_get_config".to_string(),
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

fn build_wifi_config_info(
    resp: &crate::pb::space_x::api::device::WifiGetConfigResponse,
) -> Option<WifiConfigInfo> {
    let cfg = resp.wifi_config.as_ref()?;
    let networks = cfg
        .networks
        .iter()
        .flat_map(|n| n.basic_service_sets.iter())
        .map(|bss| WifiNetworkInfo {
            band: wifi_config::Band::try_from(bss.band)
                .unwrap_or(wifi_config::Band::RfUnknown)
                .as_str_name()
                .to_string(),
            ssid: bss.ssid.clone(),
            hidden: bss.hidden,
            disabled: bss.disable,
        })
        .collect();

    Some(WifiConfigInfo {
        channel_2ghz: cfg.channel_2ghz,
        channel_5ghz: cfg.channel_5ghz,
        dfs_enabled: cfg.dfs_enabled,
        secure_dns: cfg.secure_dns,
        bypass_mode: cfg.bypass_mode,
        networks,
    })
}

fn client_info(client: &WifiClient) -> WifiClientInfo {
    let iface = wifi_client::Interface::try_from(client.iface)
        .map(|i| i.as_str_name().to_string())
        .unwrap_or_else(|_| "UNKNOWN".to_string());

    WifiClientInfo {
        name: if client.name.is_empty() {
            client.mac_address.clone()
        } else {
            client.name.clone()
        },
        mac_address: client.mac_address.clone(),
        ip_address: client.ip_address.clone(),
        signal_strength: client.signal_strength as f64,
        snr: client.snr as f64,
        associated_time_s: client.associated_time_s,
        iface,
    }
}

fn wifi_alert_labels(alerts: &WifiAlerts) -> Vec<String> {
    let mut labels = Vec::new();
    macro_rules! push_if {
        ($field:ident, $label:expr) => {
            if alerts.$field {
                labels.push($label.to_string());
            }
        };
    }

    push_if!(thermal_throttle, "Thermal throttle");
    push_if!(install_pending, "Install pending");
    push_if!(freshly_fused, "Freshly fused");
    push_if!(lan_eth_slow_link_10, "LAN ethernet link slow (10M)");
    push_if!(lan_eth_slow_link_100, "LAN ethernet link slow (100M)");
    push_if!(wan_eth_poor_connection, "WAN ethernet poor connection");
    push_if!(mesh_topology_changing_often, "Mesh topology changing often");
    push_if!(mesh_unreliable_backhaul, "Mesh backhaul unreliable");
    push_if!(radius_missing_process, "RADIUS process missing");
    push_if!(eth_switch_error, "Ethernet switch error");
    push_if!(poe_on_dish_unreachable, "PoE on dish unreachable");
    push_if!(poe_fuse_blown, "PoE fuse blown");
    push_if!(poe_router_overcurrent, "PoE router overcurrent");
    push_if!(poe_vin_overvoltage, "PoE input overvoltage");
    push_if!(poe_vin_undervoltage, "PoE input undervoltage");
    push_if!(high_cable_ping_drop_rate, "High cable ping drop rate");

    labels
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn available_actions() -> Vec<RouterAction> {
    vec![
        RouterAction {
            id: "reboot".to_string(),
            name: "Reboot Router".to_string(),
            confirm: Some("Reboot the Starlink router now? This causes temporary outage.".to_string()),
        },
        RouterAction {
            id: "checkForUpdate".to_string(),
            name: "Check for Update".to_string(),
            confirm: None,
        },
    ]
}
