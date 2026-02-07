use axum::{
    routing::get,
    Router,
};
use axum_prometheus::PrometheusMetricLayer;

pub fn setup_metrics() -> (PrometheusMetricLayer, Router) {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = Router::new().route("/metrics", get(|| async move { metric_handle.render() }));
    (prometheus_layer, app)
}
