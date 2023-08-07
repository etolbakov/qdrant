#![allow(dead_code)]

use tracing_appender::non_blocking;
use tracing_rolling_file::{RollingConditionBase, RollingFileAppender};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter, fmt, reload};

pub fn setup(
    log_level: &str,
) -> anyhow::Result<(LogLevelReloadHandle, Option<non_blocking::WorkerGuard>)> {
    tracing_log::LogTracer::init()?;

    let reg = tracing_subscriber::registry();

    let (on_disk_layer, on_disk_appender_flush_guard) = if true {
        let on_disk_appender = RollingFileAppender::new(
            "./qdrant.log",
            RollingConditionBase::new()
                .daily()
                .max_size(32 * 1024 * 1024),
            3,
        )?;

        let (on_disk_appender, on_disk_appender_flush_guard) = non_blocking(on_disk_appender);

        let on_disk_filter = filter::EnvFilter::builder().parse_lossy(log_level);

        let on_disk_layer = fmt::layer()
            .with_writer(on_disk_appender)
            .with_ansi(false)
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .with_filter(on_disk_filter);

        (Some(on_disk_layer), Some(on_disk_appender_flush_guard))
    } else {
        (None, None)
    };

    let reg = reg.with(on_disk_layer);

    let (default_filter, default_filter_handle) =
        reload::Layer::new(filter::EnvFilter::builder().parse_lossy(log_level));

    let default_layer = fmt::layer()
        .with_ansi(true)
        .with_span_events(fmt::format::FmtSpan::NEW)
        .with_filter(default_filter);

    let reg = reg.with(default_layer);

    // Use `console` or `console-subscriber` feature to enable `console-subscriber`
    //
    // Note, that `console-subscriber` requires manually enabling
    // `--cfg tokio_unstable` rust flags during compilation!
    //
    // Otherwise `console_subscriber::spawn` call panics!
    //
    // See https://docs.rs/tokio/latest/tokio/#unstable-features
    #[cfg(all(feature = "console-subscriber", tokio_unstable))]
    let reg = reg.with(console_subscriber::spawn());

    #[cfg(all(feature = "console-subscriber", not(tokio_unstable)))]
    eprintln!(
        "`console-subscriber` requires manually enabling \
         `--cfg tokio_unstable` rust flags during compilation!"
    );

    // Use `tracy` or `tracing-tracy` feature to enable `tracing-tracy`
    #[cfg(feature = "tracing-tracy")]
    let reg = reg.with(tracing_tracy::TracyLayer::new().with_filter(
        tracing_subscriber::filter::filter_fn(|metadata| metadata.is_span()),
    ));

    tracing::subscriber::set_global_default(reg)?;

    Ok((default_filter_handle.into(), on_disk_appender_flush_guard))
}

pub struct LogLevelReloadHandle(Box<dyn ReloadHandle<filter::EnvFilter>>);

impl LogLevelReloadHandle {
    pub fn get(&self) -> anyhow::Result<String> {
        todo!() // TODO!
    }

    pub fn set(&self, filter: &str) -> anyhow::Result<()> {
        let filter = filter::EnvFilter::builder().parse_lossy(filter);
        self.0.reload(filter)?;
        Ok(())
    }
}

impl From<Box<dyn ReloadHandle<filter::EnvFilter>>> for LogLevelReloadHandle {
    fn from(handle: Box<dyn ReloadHandle<filter::EnvFilter>>) -> Self {
        Self(handle)
    }
}

impl<S> From<reload::Handle<filter::EnvFilter, S>> for LogLevelReloadHandle
where
    S: 'static,
{
    fn from(handle: reload::Handle<filter::EnvFilter, S>) -> Self {
        Self(Box::new(handle))
    }
}

pub trait ReloadHandle<L>: Send + Sync {
    fn reload(&self, new_value: L) -> Result<(), reload::Error>;

    // TODO: Implement `modify` and `with_current`?
    // TODO: See https://github.com/dtolnay/erased-serde#how-it-works

    fn boxed(self) -> Box<dyn ReloadHandle<L>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl<L, S> ReloadHandle<L> for reload::Handle<L, S>
where
    L: Send + Sync,
{
    fn reload(&self, new_value: L) -> Result<(), reload::Error> {
        self.reload(new_value)
    }
}
