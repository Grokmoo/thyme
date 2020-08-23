use std::time::{Duration, Instant};

use parking_lot::{const_mutex, Mutex};

static BENCH: Mutex<BenchSet> = const_mutex(BenchSet::new());

/// A benchmarking handle created by `bench::start`.  Pass this to
/// `bench::end` to finish the given timing.
pub struct Handle {
    index: usize
}

pub fn start(tag: &str) -> Handle {
    let mut bench = BENCH.lock();
    bench.start(tag)
}

pub fn end(handle: Handle) {
    let mut bench = BENCH.lock();
    bench.end(handle);
}

pub fn report(tag: &str) -> String {
    let bench = BENCH.lock();
    bench.report(tag)
}

struct BenchSet {
    // TODO maybe use HashMap here once we can create a hashmap in const
    benches: Vec<Bench>,
}

impl BenchSet {
    const fn new() -> BenchSet {
        BenchSet {
            benches: Vec::new()
        }
    }

    fn start(&mut self, tag: &str) -> Handle {
        // TODO handle multiple starts at the same time?

        // check if bench with this tag already exists
        for (index, bench) in self.benches.iter_mut().enumerate() {
            if bench.tag == tag {
                bench.start = Some(Instant::now());
                return Handle { index };
            }
        }

        // create new bench
        let mut bench = Bench::new(tag.to_string());
        bench.start = Some(Instant::now());
        let index = self.benches.len();
        self.benches.push(bench);

        Handle { index }
    }

    fn end(&mut self, handle: Handle) {
        let bench = &mut self.benches[handle.index];
        let duration = Instant::now() - bench.start.take().unwrap_or_else(Instant::now);
        bench.history.push(duration);
    }

    fn report(&self, tag: &str) -> String {
        for bench in self.benches.iter() {
            if bench.tag == tag {
                return bench.report_str();
            }
        }

        "Bench not found".to_string()
    }
}

struct Bench {
    tag: String,
    history: Vec<Duration>,
    start: Option<Instant>,
}

const MOVING_AVG_LEN: usize = 30;

impl Bench {
    fn new(tag: String) -> Bench {
        Bench {
            history: Vec::new(),
            start: None,
            tag,
        }
    }

    fn stats(&self) -> (f32, f32, f32) {
        let count = std::cmp::min(MOVING_AVG_LEN, self.history.len());

        let data = || { self.history.iter().rev().take(MOVING_AVG_LEN) };

        let sum = (data)().sum::<Duration>().as_secs_f32();
        let max = match (data)().max() {
            None => 0.0,
            Some(dur) => dur.as_secs_f32(),
        };

        let avg = sum / (count as f32);

        let numer: f32 = (data)().map(|d| (d.as_secs_f32() - avg) * (d.as_secs_f32() - avg)).sum();

        let stdev_sq = numer / (count as f32 - 1.0);
        let stdev = stdev_sq.sqrt();

        (avg * 1000.0, stdev * 1000.0, max * 1000.0)
    }

    fn report_str(&self) -> String {
        let (avg, err, max) = self.stats();
        format!(
            "{}: {:.2} ± {:.2}; max {:.2} ms",
            self.tag, avg, err, max,
        )
    }
}
