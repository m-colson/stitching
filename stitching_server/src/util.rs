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

pub fn ws_upgrader<M, S: Send + Sync + Clone + 'static, Fut>(
    cb: impl FnOnce(S, WebSocket) -> Fut + Send + Sync + Clone + 'static,
) -> impl Handler<(M, State<S>, WebSocketUpgrade), S>
where
    WebSocketUpgrade: FromRequest<S, M>,
    Fut: Future<Output = ()> + Send + 'static,
{
    |State(state), ws: WebSocketUpgrade| async move { ws.on_upgrade(move |sock| cb(state, sock)) }
}

pub struct IntervalTimer {
    base_time: Instant,
    mark_time: Instant,
}

impl IntervalTimer {
    #[inline]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            base_time: now,
            mark_time: now,
        }
    }

    #[inline]
    pub fn start(&mut self) {
        let now = Instant::now();
        self.base_time = now;
        self.mark_time = now;
    }

    #[inline]
    pub fn mark(&mut self, name: &str) {
        let now = Instant::now();
        let took = now - self.mark_time;
        Metrics::push(name, took.as_secs_f64() * 1000.);

        // let took = format!("{:.1?}", took);
        // tracing::debug!(took, "{}", name);

        self.mark_time = now;
    }

    #[inline]
    pub fn mark_from_base(&mut self, name: &str) {
        let now = Instant::now();
        let took = now - self.base_time;
        Metrics::push(name, took.as_secs_f64() * 1000.);

        // let took = format!("{took:.1?}");
        // tracing::info!(took, "{}", name);

        self.mark_time = now;
    }

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

    pub fn push(name: &str, v: f64) {
        Self::lock_global()
            .marks
            .entry(name.to_string())
            .or_default()
            .push(v);
    }

    pub fn current_marks() -> HashMap<String, (f64, f64, usize)> {
        Self::lock_global()
            .marks
            .iter()
            .map(|(k, v)| (k.clone(), (v.average(), v.std_dev(), v.len())))
            .collect()
    }

    pub fn reset() {
        Self::lock_global().marks = HashMap::new();
    }

    pub fn save_csv(out_path: impl AsRef<path::Path>) -> io::Result<()> {
        let mut out = fs::File::create(out_path)?;

        writeln!(out, "name,mean,stddev,samples")?;
        let mut marks = Self::current_marks().into_iter().collect::<Vec<_>>();
        marks.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (name, (mean, stddev, count)) in marks {
            writeln!(out, "{name},{mean:.2},{stddev:.2},{count}")?;
        }

        Ok(())
    }

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

#[derive(Clone, Copy, Default)]
struct Metric {
    sum: f64,
    sum_sq: f64,
    count: u32,
}

impl Metric {
    #[inline]
    pub fn push(&mut self, v: f64) {
        self.sum += v;
        self.sum_sq += v * v;
        self.count += 1;
    }

    #[inline]
    pub fn average(self) -> f64 {
        self.sum / f64::from(self.count)
    }

    #[inline]
    pub fn std_dev(self) -> f64 {
        let n = f64::from(self.count);
        let exp_x = self.sum / n;
        let exp_x2 = self.sum_sq / n;
        exp_x.mul_add(-exp_x, exp_x2).sqrt()
    }

    #[inline]
    pub const fn len(self) -> usize {
        self.count as _
    }
}
