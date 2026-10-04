// FLUX_PERF=1: každé 2 s na stderr koľko trvá jedna snímka (čas CPU v ui()) – na meranie plynulosti
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::Instant;

static ON: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static TOTAL_US: AtomicU64 = AtomicU64::new(0);
static MAX_US: AtomicU64 = AtomicU64::new(0);
static LAST: Mutex<Option<Instant>> = Mutex::new(None);

pub fn init() {
    ON.store(std::env::var("FLUX_PERF").as_deref() == Ok("1"), Relaxed);
}

pub struct Frame(Option<Instant>);

pub fn frame() -> Frame {
    Frame(ON.load(Relaxed).then(Instant::now))
}

impl Drop for Frame {
    fn drop(&mut self) {
        let Some(t) = self.0 else { return };
        let us = t.elapsed().as_micros() as u64;
        FRAMES.fetch_add(1, Relaxed);
        TOTAL_US.fetch_add(us, Relaxed);
        MAX_US.fetch_max(us, Relaxed);
        let mut last = LAST.lock().unwrap();
        let since = last.get_or_insert(t);
        if since.elapsed().as_secs_f32() >= 2.0 {
            let n = FRAMES.swap(0, Relaxed).max(1);
            let total = TOTAL_US.swap(0, Relaxed);
            let max = MAX_US.swap(0, Relaxed);
            eprintln!("perf: frames {n}, avg {:.2} ms, max {:.2} ms", total as f64 / n as f64 / 1000.0, max as f64 / 1000.0);
            *last = Some(Instant::now());
        }
    }
}
