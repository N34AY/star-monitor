use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

use crate::collector_config;
use crate::dish;
use crate::router;

const DB_FILE: &str = "history.sqlite";
const OUTAGE_SYNC_INTERVAL_S: u64 = 60;
const CLEANUP_INTERVAL_S: u64 = 3600;

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create app data dir: {e}"))?;
    Ok(dir.join(DB_FILE))
}

fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let path = db_path(app)?;
    let conn = Connection::open(path).map_err(|e| format!("failed to open history db: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("failed to set WAL mode: {e}"))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts_unix_ms INTEGER NOT NULL,
            source TEXT NOT NULL,
            reachable INTEGER NOT NULL,
            uptime_s INTEGER,
            latency_ms REAL,
            ping_drop_percent REAL,
            downlink_mbps REAL,
            uplink_mbps REAL,
            reboot_reason TEXT,
            disablement_code TEXT,
            alerts TEXT,
            error TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_samples_ts ON samples(source, ts_unix_ms);

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts_unix_ms INTEGER NOT NULL,
            source TEXT NOT NULL,
            kind TEXT NOT NULL,
            detail TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_events_ts ON events(source, ts_unix_ms);",
    )
    .map_err(|e| format!("failed to init history schema: {e}"))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---- writing samples/events ----------------------------------------------

struct SampleInput<'a> {
    source: &'a str,
    reachable: bool,
    uptime_s: Option<u64>,
    latency_ms: Option<f64>,
    ping_drop_percent: Option<f64>,
    downlink_mbps: Option<f64>,
    uplink_mbps: Option<f64>,
    reboot_reason: Option<&'a str>,
    disablement_code: Option<&'a str>,
    alerts: &'a [String],
    error: Option<&'a str>,
}

fn insert_sample(conn: &Connection, ts_ms: i64, s: &SampleInput) -> Result<(), String> {
    let alerts_json = serde_json::to_string(s.alerts).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO samples (ts_unix_ms, source, reachable, uptime_s, latency_ms, ping_drop_percent, downlink_mbps, uplink_mbps, reboot_reason, disablement_code, alerts, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            ts_ms,
            s.source,
            s.reachable as i32,
            s.uptime_s.map(|v| v as i64),
            s.latency_ms,
            s.ping_drop_percent,
            s.downlink_mbps,
            s.uplink_mbps,
            s.reboot_reason,
            s.disablement_code,
            alerts_json,
            s.error,
        ],
    )
    .map_err(|e| format!("failed to insert sample: {e}"))?;
    Ok(())
}

fn insert_event(conn: &Connection, ts_ms: i64, source: &str, kind: &str, detail: &str) {
    let result = conn.execute(
        "INSERT INTO events (ts_unix_ms, source, kind, detail) VALUES (?1, ?2, ?3, ?4)",
        params![ts_ms, source, kind, detail],
    );
    if let Err(e) = result {
        eprintln!("history collector: failed to insert event: {e}");
    }
}

fn dish_outage_exists(conn: &Connection, ts_ms: i64) -> bool {
    conn.query_row(
        "SELECT 1 FROM events WHERE source = 'dish' AND kind = 'dish_outage' AND ts_unix_ms = ?1 LIMIT 1",
        params![ts_ms],
        |_| Ok(()),
    )
    .is_ok()
}

// ---- reachability/reboot/alert diffing ------------------------------------

struct SourceState {
    initialized: bool,
    reachable: bool,
    uptime_s: Option<u64>,
    alerts: Vec<String>,
    unreachable_since_ms: Option<i64>,
}

impl Default for SourceState {
    fn default() -> Self {
        Self {
            initialized: false,
            reachable: true,
            uptime_s: None,
            alerts: Vec::new(),
            unreachable_since_ms: None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn diff_and_emit(
    conn: &Connection,
    source: &str,
    ts_ms: i64,
    reachable: bool,
    uptime_s: Option<u64>,
    alerts: &[String],
    reboot_reason: Option<&str>,
    state: &mut SourceState,
    poll_interval_s: u32,
) {
    if !state.initialized {
        state.initialized = true;
        state.reachable = reachable;
        state.uptime_s = uptime_s;
        state.alerts = alerts.to_vec();
        state.unreachable_since_ms = if reachable { None } else { Some(ts_ms) };
        return;
    }

    if reachable && !state.reachable {
        let down_s = state
            .unreachable_since_ms
            .map(|since| (ts_ms - since) / 1000)
            .unwrap_or(0);
        insert_event(
            conn,
            ts_ms,
            source,
            "reachable_again",
            &format!("back online after ~{down_s}s unreachable"),
        );
        state.unreachable_since_ms = None;
    } else if !reachable && state.reachable {
        state.unreachable_since_ms = Some(ts_ms);
        insert_event(conn, ts_ms, source, "unreachable", "stopped responding");
    }

    if reachable {
        // Uptime dropping by more than a poll-interval's worth of slack means
        // the device rebooted between the previous poll and this one.
        if let (Some(prev_up), Some(cur_up)) = (state.uptime_s, uptime_s) {
            let tolerance = (poll_interval_s as u64).max(3);
            if cur_up + tolerance < prev_up {
                let reason = reboot_reason.unwrap_or("UNKNOWN");
                insert_event(
                    conn,
                    ts_ms,
                    source,
                    "reboot_detected",
                    &format!("reboot detected (reason={reason}, uptime reset to {cur_up}s)"),
                );
            }
        }
        state.uptime_s = uptime_s;

        let prev: HashSet<&String> = state.alerts.iter().collect();
        let cur: HashSet<&String> = alerts.iter().collect();
        for raised in cur.difference(&prev) {
            insert_event(conn, ts_ms, source, "alert_raised", raised);
        }
        for cleared in prev.difference(&cur) {
            insert_event(conn, ts_ms, source, "alert_cleared", cleared);
        }
        state.alerts = alerts.to_vec();
    }

    state.reachable = reachable;
}

fn cleanup(conn: &Connection, retention_days: u32) {
    let cutoff_ms = now_unix_ms() - (retention_days.max(1) as i64) * 86_400_000;
    for table in ["samples", "events"] {
        let sql = format!("DELETE FROM {table} WHERE ts_unix_ms < ?1");
        if let Err(e) = conn.execute(&sql, params![cutoff_ms]) {
            eprintln!("history collector: cleanup of {table} failed: {e}");
        }
    }
}

// ---- collector loop --------------------------------------------------------

#[allow(clippy::type_complexity)]
fn dish_fields(
    overview: &Option<dish::DishOverview>,
) -> (
    Option<u64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<String>,
    Option<String>,
    Vec<String>,
) {
    match overview {
        Some(ov) => (
            ov.key_metrics.uptime_s,
            ov.key_metrics.latency_ms,
            ov.key_metrics.ping_drop_percent,
            ov.key_metrics.downlink_mbps,
            ov.key_metrics.uplink_mbps,
            Some(ov.reboot_reason.clone()),
            Some(ov.disablement_code.clone()),
            ov.alerts.clone(),
        ),
        None => (None, None, None, None, None, None, None, Vec::new()),
    }
}

fn router_fields(
    overview: &Option<router::RouterOverview>,
) -> (Option<u64>, Option<f64>, Option<f64>, Vec<String>) {
    match overview {
        Some(ov) => (
            ov.uptime_s,
            Some(ov.ping_latency_ms),
            Some(ov.ping_drop_percent),
            ov.alerts.clone(),
        ),
        None => (None, None, None, Vec::new()),
    }
}

pub async fn run_collector(app: AppHandle) {
    let conn = loop {
        match open_db(&app) {
            Ok(c) => break c,
            Err(e) => {
                eprintln!("history collector: failed to open db: {e}");
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
    };

    let mut dish_state = SourceState::default();
    let mut router_state = SourceState::default();
    let mut last_outage_sync = Instant::now() - Duration::from_secs(OUTAGE_SYNC_INTERVAL_S);
    let mut last_cleanup = Instant::now() - Duration::from_secs(CLEANUP_INTERVAL_S);

    loop {
        let cfg = collector_config::load(&app);
        let poll = cfg.poll_interval_s.max(2);
        let now_ms = now_unix_ms();

        if cfg.dish_enabled {
            let probe = dish::probe_dish_status(&cfg.dish_address).await;
            let (uptime, latency, drop, dl, ul, reboot_reason, disablement, alerts) =
                dish_fields(&probe.overview);

            let _ = insert_sample(
                &conn,
                now_ms,
                &SampleInput {
                    source: "dish",
                    reachable: probe.reachable,
                    uptime_s: uptime,
                    latency_ms: latency,
                    ping_drop_percent: drop,
                    downlink_mbps: dl,
                    uplink_mbps: ul,
                    reboot_reason: reboot_reason.as_deref(),
                    disablement_code: disablement.as_deref(),
                    alerts: &alerts,
                    error: probe.error.as_deref(),
                },
            );
            diff_and_emit(
                &conn,
                "dish",
                now_ms,
                probe.reachable,
                uptime,
                &alerts,
                reboot_reason.as_deref(),
                &mut dish_state,
                poll,
            );

            if last_outage_sync.elapsed().as_secs() >= OUTAGE_SYNC_INTERVAL_S {
                if let Ok(hist) = dish::fetch_dish_history(Some(cfg.dish_address.clone())).await {
                    for o in hist.outages {
                        let ts_ms = o.start_unix_s * 1000;
                        if !dish_outage_exists(&conn, ts_ms) {
                            insert_event(
                                &conn,
                                ts_ms,
                                "dish",
                                "dish_outage",
                                &format!("{} for {:.0}s", o.cause, o.duration_s),
                            );
                        }
                    }
                }
                last_outage_sync = Instant::now();
            }
        }

        if cfg.router_enabled {
            let probe = router::probe_router_status(&cfg.router_address).await;
            let (uptime, latency, drop, alerts) = router_fields(&probe.overview);

            let _ = insert_sample(
                &conn,
                now_ms,
                &SampleInput {
                    source: "router",
                    reachable: probe.reachable,
                    uptime_s: uptime,
                    latency_ms: latency,
                    ping_drop_percent: drop,
                    downlink_mbps: None,
                    uplink_mbps: None,
                    reboot_reason: None,
                    disablement_code: None,
                    alerts: &alerts,
                    error: probe.error.as_deref(),
                },
            );
            diff_and_emit(
                &conn,
                "router",
                now_ms,
                probe.reachable,
                uptime,
                &alerts,
                None,
                &mut router_state,
                poll,
            );
        }

        if last_cleanup.elapsed().as_secs() >= CLEANUP_INTERVAL_S {
            cleanup(&conn, cfg.retention_days);
            last_cleanup = Instant::now();
        }

        tokio::time::sleep(Duration::from_secs(poll as u64)).await;
    }
}

// ---- read-side commands for the frontend -----------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEvent {
    pub ts_unix_ms: i64,
    pub source: String,
    pub kind: String,
    pub detail: String,
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<HistoryEvent> {
    Ok(HistoryEvent {
        ts_unix_ms: row.get(0)?,
        source: row.get(1)?,
        kind: row.get(2)?,
        detail: row.get(3)?,
    })
}

#[tauri::command]
pub fn query_events(
    app: AppHandle,
    source: Option<String>,
    since_unix_ms: Option<i64>,
    limit: Option<u32>,
) -> Result<Vec<HistoryEvent>, String> {
    let conn = open_db(&app)?;
    let limit = limit.unwrap_or(200).min(2000);
    let since = since_unix_ms.unwrap_or(0);

    let mut stmt = match &source {
        Some(_) => conn
            .prepare(
                "SELECT ts_unix_ms, source, kind, detail FROM events
                 WHERE source = ?1 AND ts_unix_ms >= ?2
                 ORDER BY ts_unix_ms DESC LIMIT ?3",
            )
            .map_err(|e| format!("query failed: {e}"))?,
        None => conn
            .prepare(
                "SELECT ts_unix_ms, source, kind, detail FROM events
                 WHERE ts_unix_ms >= ?1
                 ORDER BY ts_unix_ms DESC LIMIT ?2",
            )
            .map_err(|e| format!("query failed: {e}"))?,
    };

    let rows = match &source {
        Some(src) => stmt.query_map(params![src, since, limit], row_to_event),
        None => stmt.query_map(params![since, limit], row_to_event),
    }
    .map_err(|e| format!("query failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("query failed: {e}"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySamplePoint {
    pub ts_unix_ms: i64,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub ping_drop_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySamples {
    pub points: Vec<HistorySamplePoint>,
}

// Same stride-decimation idea as `decimate()` in dish.rs/router.rs, just
// applied to timestamped rows read back from SQLite instead of a flat array.
fn decimate_points(points: Vec<HistorySamplePoint>, max_points: usize) -> Vec<HistorySamplePoint> {
    if max_points == 0 || points.len() <= max_points {
        return points;
    }
    let stride = ((points.len() as f64) / (max_points as f64)).ceil() as usize;
    points.into_iter().step_by(stride.max(1)).collect()
}

#[tauri::command]
pub fn query_samples(
    app: AppHandle,
    source: String,
    since_unix_ms: i64,
    max_points: Option<u32>,
) -> Result<HistorySamples, String> {
    let conn = open_db(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT ts_unix_ms, reachable, latency_ms, ping_drop_percent FROM samples
             WHERE source = ?1 AND ts_unix_ms >= ?2
             ORDER BY ts_unix_ms ASC",
        )
        .map_err(|e| format!("query failed: {e}"))?;

    let rows = stmt
        .query_map(params![source, since_unix_ms], |row| {
            Ok(HistorySamplePoint {
                ts_unix_ms: row.get(0)?,
                reachable: row.get::<_, i64>(1)? != 0,
                latency_ms: row.get(2)?,
                ping_drop_percent: row.get(3)?,
            })
        })
        .map_err(|e| format!("query failed: {e}"))?;

    let points: Vec<HistorySamplePoint> = rows
        .collect::<Result<_, _>>()
        .map_err(|e| format!("query failed: {e}"))?;

    let max_points = max_points.unwrap_or(300) as usize;
    Ok(HistorySamples {
        points: decimate_points(points, max_points),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorStatus {
    pub running: bool,
    pub last_poll_unix_ms: Option<i64>,
    pub sample_count: i64,
    pub event_count: i64,
}

#[tauri::command]
pub fn get_collector_status(app: AppHandle) -> Result<CollectorStatus, String> {
    let conn = open_db(&app)?;
    let cfg = collector_config::load(&app);

    let sample_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM samples", [], |r| r.get(0))
        .map_err(|e| format!("query failed: {e}"))?;
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .map_err(|e| format!("query failed: {e}"))?;
    let last_poll_unix_ms: Option<i64> = conn
        .query_row("SELECT MAX(ts_unix_ms) FROM samples", [], |r| r.get(0))
        .unwrap_or(None);

    Ok(CollectorStatus {
        running: cfg.dish_enabled || cfg.router_enabled,
        last_poll_unix_ms,
        sample_count,
        event_count,
    })
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    let conn = open_db(&app)?;
    conn.execute_batch("DELETE FROM samples; DELETE FROM events; VACUUM;")
        .map_err(|e| format!("failed to clear history: {e}"))
}
