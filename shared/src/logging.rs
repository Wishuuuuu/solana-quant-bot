use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

pub fn init_logging(component: &str) {
    let log_dir = "logs";
    std::fs::create_dir_all(log_dir).expect("Failed to create log directory");

    // File appender with daily rotation
    let file_appender = rolling::daily(log_dir, component);
    let (non_blocking, _guard) = non_blocking(file_appender);

    // Console and file logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .with_ansi(false)
                .with_target(true)
        )
        .init();
}

pub fn init_logging_with_level(component: &str, level: &str) {
    let log_dir = "logs";
    std::fs::create_dir_all(log_dir).expect("Failed to create log directory");

    let file_appender = rolling::daily(log_dir, component);
    let (non_blocking, _guard) = non_blocking(file_appender);

    let env_filter = EnvFilter::new(format!("{},{}=debug", level, component));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .with_ansi(false)
        )
        .init();
}
