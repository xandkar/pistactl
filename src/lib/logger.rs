use tracing_subscriber::{filter, EnvFilter, FmtSubscriber};

pub fn init(debug: bool) -> anyhow::Result<()> {
    let level = if debug {
        filter::LevelFilter::DEBUG.into()
    } else {
        filter::LevelFilter::INFO.into()
    };
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(level)
                .from_env()?,
        )
        .with_writer(std::io::stderr)
        .with_target(debug)
        .with_file(debug)
        .with_line_number(debug)
        .with_timer(
            // Specifying format manually instead of just using
            //
            //     tracing_subscriber::fmt::time::LocalTime::rfc_3339()
            //
            // because rfc_3339() ends up printing microseconds and causes
            // variable width lines which do not align.
            tracing_subscriber::fmt::time::LocalTime::new(
            time::macros::format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second][offset_hour]:[offset_minute]"
            ),
        ))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
