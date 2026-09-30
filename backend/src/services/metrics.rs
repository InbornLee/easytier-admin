use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Duration;

use futures_util::future::join_all;
use serde::Serialize;

use crate::state::{AppState, SharedState};
use crate::util::format::parse_human_size;
use crate::util::now_ms;

const MAX_POINTS: usize = 480;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
pub struct TrafficPoint {
    pub t: i64,
    pub rx: f64,
    pub tx: f64,
}

#[derive(Default)]
pub struct MetricsStore {
    global: VecDeque<TrafficPoint>,
    by_network: HashMap<String, VecDeque<TrafficPoint>>,
}

fn push_point(series: &mut VecDeque<TrafficPoint>, point: TrafficPoint) {
    series.push_back(point);
    while series.len() > MAX_POINTS {
        series.pop_front();
    }
}

impl MetricsStore {
    fn push_global(&mut self, point: TrafficPoint) {
        push_point(&mut self.global, point);
    }

    fn push_network(&mut self, network_id: &str, point: TrafficPoint) {
        let series = self.by_network.entry(network_id.to_string()).or_default();
        push_point(series, point);
    }

    pub fn get_series(&self, network_id: Option<&str>, since: Option<i64>) -> Vec<TrafficPoint> {
        let series = match network_id {
            Some(id) => self.by_network.get(id),
            None => Some(&self.global),
        };
        match series {
            Some(points) => points
                .iter()
                .filter(|p| since.map(|s| p.t >= s).unwrap_or(true))
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn remove_network(&mut self, network_id: &str) {
        self.by_network.remove(network_id);
    }

    /// 汇总指定网络集合的流量序列（按采样时间点相加）。
    pub fn sum_series(
        &self,
        network_ids: &HashSet<String>,
        since: Option<i64>,
    ) -> Vec<TrafficPoint> {
        let mut buckets: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for id in network_ids {
            let Some(series) = self.by_network.get(id) else {
                continue;
            };
            for point in series {
                if since.map(|s| point.t >= s).unwrap_or(true) {
                    let entry = buckets.entry(point.t).or_insert((0.0, 0.0));
                    entry.0 += point.rx;
                    entry.1 += point.tx;
                }
            }
        }
        buckets
            .into_iter()
            .map(|(t, (rx, tx))| TrafficPoint { t, rx, tx })
            .collect()
    }
}

pub fn start_metrics_collector(state: &SharedState) {
    let state = state.clone();
    tokio::spawn(async move {
        sample(&state).await;
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            sample(&state).await;
        }
    });
}

async fn sample(state: &AppState) {
    let rows = match state.db.list_networks() {
        Ok(rows) => rows,
        Err(_) => return,
    };
    let ts = now_ms();
    let mut futures = Vec::new();
    for row in rows {
        if !state.pm.is_running(&row.id) {
            continue;
        }
        let cli = state.cli(row.rpc_port as u16, Duration::from_millis(4000));
        let network_id = row.id.clone();
        futures.push(async move {
            match cli.peer_list().await {
                Ok(peers) => {
                    let mut rx = 0.0;
                    let mut tx = 0.0;
                    for p in &peers {
                        rx += parse_human_size(Some(&p.rx_bytes));
                        tx += parse_human_size(Some(&p.tx_bytes));
                    }
                    (network_id, rx, tx, true)
                }
                Err(_) => (network_id, 0.0, 0.0, false),
            }
        });
    }
    let results = join_all(futures).await;
    let mut global_rx = 0.0;
    let mut global_tx = 0.0;
    let mut store = state.metrics.lock().unwrap();
    for (network_id, rx, tx, ok) in results {
        if ok {
            store.push_network(&network_id, TrafficPoint { t: ts, rx, tx });
            global_rx += rx;
            global_tx += tx;
        }
    }
    store.push_global(TrafficPoint {
        t: ts,
        rx: global_rx,
        tx: global_tx,
    });
}

pub fn get_traffic_series(
    state: &AppState,
    network_id: Option<&str>,
    since: Option<i64>,
) -> Vec<TrafficPoint> {
    state.metrics.lock().unwrap().get_series(network_id, since)
}

pub fn remove_network_series(state: &AppState, network_id: &str) {
    state.metrics.lock().unwrap().remove_network(network_id);
}
