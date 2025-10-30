use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use time::macros::format_description;

pub fn init() {
    // Default to info if RUST_LOG not set
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("env filter");

    // RFC3339-like with UTC
    let timer = UtcTime::new(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"));

    let fmt_layer = fmt::layer()
        .event_format(fmt::format().json().with_current_span(false).with_span_list(false))
        .fmt_fields(fmt::format::JsonFields::new())
        .with_timer(timer)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

pub fn short_uuid() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    id.split('-').next().unwrap_or(&id).to_string()
}

pub mod events {
    pub const APP_START: &str = "app_start";
    pub const APP_READY: &str = "app_ready";
    pub const APP_SHUTDOWN: &str = "app_shutdown";

    pub const HTTP_REQUEST: &str = "http_request";
    pub const HTTP_RESPONSE: &str = "http_response";

    pub const RELAY_START: &str = "relay_start";
    pub const RELAY_STOP: &str = "relay_stop";
    pub const RELAY_ERROR: &str = "relay_error";

    pub const PEER_CONNECTED: &str = "peer_connected";
    pub const PEER_DISCONNECTED: &str = "peer_disconnected";

    pub const RECONNECT_SCHEDULED: &str = "reconnect_scheduled";
    pub const RECONNECT_ATTEMPT: &str = "reconnect_attempt";
    pub const RECONNECT_SUCCESS: &str = "reconnect_success";
    pub const RECONNECT_GIVEUP: &str = "reconnect_giveup";
}
