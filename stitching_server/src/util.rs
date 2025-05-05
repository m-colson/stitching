//! This module contains general utilities for logging and web sockets.

use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    future::Future,
    io::{self, Write},
    path,
    sync::{LazyLock, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use axum::{
    extract::{FromRequest, State, WebSocketUpgrade, ws::WebSocket},
    handler::Handler,
};

/// Creates a [`Handler`] that will call `cb` when a websocket connection is made.
pub fn ws_upgrader<M, S: Send + Sync + Clone + 'static, Fut>(
    cb: impl FnOnce(S, WebSocket) -> Fut + Send + Sync + Clone + 'static,
) -> impl Handler<(M, State<S>, WebSocketUpgrade), S>
where
    WebSocketUpgrade: FromRequest<S, M>,
    Fut: Future<Output = ()> + Send + 'static,
{
    |State(state), ws: WebSocketUpgrade| async move { ws.on_upgrade(move |sock| cb(state, sock)) }
}

/// Stores the information necessary to determine how long something took.
/// Contains a base time which will not be changed on [`IntervalTimer::mark`] and
/// can be used to record how long multiple markings took.
pub struct IntervalTimer {
    base_time: Instant,
    mark_time: Instant,
}

impl IntervalTimer {
    /// Creates a timer with the current time as its basis.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            base_time: now,
            mark_time: now,
        }
    }

    /// Resets the basis and last marking time to the current time.
    #[inline]
    pub fn start(&mut self) {
        let now = Instant::now();
        self.base_time = now;
        self.mark_time = now;
    }

    /// Determines the time since the last marking and records a metric with `name`.
    /// See [`Metrics::push`].
    #[inline]
    pub fn mark(&mut self, name: &str) {
        let now = Instant::now();
        let took = now - self.mark_time;
        Metrics::push(name, took.as_secs_f64() * 1000.);

        // let took = format!("{:.1?}", took);
        // tracing::debug!(took, "{}", name);

        self.mark_time = now;
    }

    /// Determines the time since the timer basis and records a metric with `name`.
    /// See [`Metrics::push`].
    #[inline]
    pub fn mark_from_base(&mut self, name: &str) {
        let now = Instant::now();
        let took = now - self.base_time;
        Metrics::push(name, took.as_secs_f64() * 1000.);

        // let took = format!("{took:.1?}");
        // tracing::info!(took, "{}", name);

        self.mark_time = now;
    }

    /// Determines the time since the timer basis and records a metric with `name`.
    /// If less time has elapsed than the target expects,
    /// it will sleep until the target is reached.
    #[inline]
    pub async fn log_and_wait_fps(&self, name: &str, target: Duration) {
        let diff = self.base_time.elapsed();
        Metrics::push(name, diff.as_secs_f64() * 1000.);

        if target > diff {
            tokio::time::sleep(target - diff).await;
            Metrics::push(
                &format!("{name}+sleep"),
                self.base_time.elapsed().as_secs_f64() * 1000.,
            );
        }

        // let fps = format!("{:.1}", 1. / diff.as_secs_f32());
        // let took = format!("{diff:.1?}");
        // tracing::info!(fps, took, "{}", name);
    }
}

static GLOBAL_METRICS: LazyLock<Mutex<Metrics>> = LazyLock::new(|| Mutex::new(Metrics::new()));

/// Contains map of named [`Metric`]s.
/// In this software, a singleton of this type is used for all methods of this type.
pub struct Metrics {
    marks: HashMap<String, Metric>,
}

impl Metrics {
    fn new() -> Self {
        Self {
            marks: HashMap::new(),
        }
    }

    fn lock_global() -> MutexGuard<'static, Self> {
        match GLOBAL_METRICS.lock() {
            Ok(g) => g,
            Err(mut err) => {
                **err.get_mut() = Metrics::new();
                err.into_inner()
            }
        }
    }

    /// [`Metric::push`] to the value `v` with the given `name`.
    /// If the name has not been used yet, it will be created.
    pub fn push(name: &str, v: f64) {
        Self::lock_global()
            .marks
            .entry(name.to_string())
            .or_default()
            .push(v);
    }

    /// Gets the current marking names and metric (average, standard deviation, count).
    pub fn current_marks() -> HashMap<String, (f64, f64, usize)> {
        Self::lock_global()
            .marks
            .iter()
            .map(|(k, v)| (k.clone(), (v.average(), v.std_dev(), v.len())))
            .collect()
    }

    /// Clears all saved metrics.
    pub fn reset() {
        Self::lock_global().marks = HashMap::new();
    }

    /// Creates and saves the current metrics to a csv file at `out_path`.
    pub fn write_csv(out_path: impl AsRef<path::Path>) -> io::Result<()> {
        let mut out = fs::File::create(out_path)?;

        writeln!(out, "time,name,mean,stddev,samples")?;
        let mut marks = Self::current_marks().into_iter().collect::<Vec<_>>();
        marks.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (name, (mean, stddev, count)) in marks {
            writeln!(
                out,
                "{},{name},{mean:.2},{stddev:.2},{count}",
                chrono::Local::now()
            )?;
        }

        Ok(())
    }

    /// Runs the callback with an immutable reference to the singleton `Metrics` instance.
    pub fn with(f: impl FnOnce(&Self)) {
        f(&Self::lock_global())
    }
}

impl Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut metrics = self.marks.iter().collect::<Vec<_>>();
        metrics.sort_by_key(|(n, _)| *n);

        let mut write_comma = false;
        for (n, m) in metrics {
            if write_comma {
                f.write_str(", ")?;
            }

            write!(f, "{n} = {:.1?}σ{:.1?}", m.average(), m.std_dev())?;

            write_comma = true;
        }
        Ok(())
    }
}

/// Represents a list of `f64` values in a way that is efficent to find the
/// average and standard deviation of the list.
#[derive(Clone, Copy, Default)]
pub struct Metric {
    sum: f64,
    sum_sq: f64,
    count: u32,
}

impl Metric {
    /// Adds the value to the pseudo-list.
    #[inline]
    pub fn push(&mut self, v: f64) {
        self.sum += v;
        self.sum_sq += v * v;
        self.count += 1;
    }

    /// Calculates the average of the pseudo-list.
    #[inline]
    pub fn average(self) -> f64 {
        self.sum / f64::from(self.count)
    }

    /// Calculates the standard deviation of the pseudo-list.
    #[inline]
    pub fn std_dev(self) -> f64 {
        let n = f64::from(self.count);
        let exp_x = self.sum / n;
        let exp_x2 = self.sum_sq / n;
        exp_x.mul_add(-exp_x, exp_x2).sqrt()
    }

    /// Returns the number of values in the pseudo-list.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(self) -> usize {
        self.count as _
    }
}
