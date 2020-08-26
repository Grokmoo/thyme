use std::time::{Duration, Instant};

use parking_lot::{const_mutex, Mutex};

static BENCH: Mutex<BenchSet> = const_mutex(BenchSet::new());

/// A benchmarking handle created by `bench::start`.  `end` this to
/// finish the given benchmark timing
pub struct Handle {
    index: usize
}

impl Handle {
    pub fn end(self) {
        end(self);
    }
}

pub fn run<F: FnOnce()>(tag: &str, block: F) {
    let handle = start(tag);
    (block)();
    end(handle);
}

pub fn start(tag: &str) -> Handle {
    let mut bench = BENCH.lock();
    bench.start(tag)
}

pub fn end(handle: Handle) {
    let mut bench = BENCH.lock();
    bench.end(handle);
}

pub fn stats(tag: &str) -> Stats {
    let bench = BENCH.lock();
    bench.stats(tag)
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

    fn stats(&self, tag: &str) -> Stats {
        for bench in self.benches.iter() {
            if bench.tag == tag {
                return bench.stats();
            }
        }

        Stats::default()
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

#[derive(Debug, Copy, Clone)]
pub struct Stats {
    average_s: f32,
    stdev_s: f32,
    max_s: f32,
    unit: Unit,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            average_s: 0.0,
            stdev_s: 0.0,
            max_s: 0.0,
            unit: Unit::Seconds,
        }
    }
}

impl Stats {
    pub fn average(&self) -> f32 {
        self.average_s * self.unit.multiplier()
    }

    pub fn stdev(&self) -> f32 {
        self.stdev_s * self.unit.multiplier()
    }

    pub fn max(&self) -> f32 {
        self.max_s * self.unit.multiplier()
    }

    pub fn unit_postfix(&self) -> &'static str {
        self.unit.postfix()
    }

    pub fn pick_unit(self) -> Stats {
        const CHANGE_VALUE: f32 = 0.0999999;
        
        if self.average_s > CHANGE_VALUE {
            self.in_seconds()
        } else if self.average_s * Unit::Millis.multiplier() > CHANGE_VALUE {
            self.in_millis()
        } else {
            self.in_micros()
        }
    }

    pub fn in_seconds(self) -> Stats {
        Stats {
            average_s: self.average_s,
            stdev_s: self.stdev_s,
            max_s: self.max_s,
            unit: Unit::Seconds,
        }
    }

    pub fn in_millis(self) -> Stats {
        Stats {
            average_s: self.average_s,
            stdev_s: self.stdev_s,
            max_s: self.max_s,
            unit: Unit::Millis,
        }
    }

    pub fn in_micros(self) -> Stats {
        Stats {
            average_s: self.average_s,
            stdev_s: self.stdev_s,
            max_s: self.max_s,
            unit: Unit::Micros,
        }
    }
}


#[derive(Copy, Clone, Debug)]
enum Unit {
    Seconds,
    Millis,
    Micros,
}

impl Unit {
    fn postfix(self) -> &'static str {
        use Unit::*;
        match self {
            Seconds => "s",
            Millis => "ms",
            Micros => "µs"
        }
    }

    fn multiplier(self) -> f32 {
        use Unit::*;
        match self {
            Seconds => 1.0,
            Millis => 1000.0,
            Micros => 1_000_000.0,
        }
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

    fn stats(&self) -> Stats {
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

        Stats {
            average_s: avg,
            stdev_s: stdev,
            max_s: max,
            unit: Unit::Seconds,
        }
    }

    fn report_str(&self) -> String {
        let stats = self.stats().pick_unit();
        format!(
            "{}: {:.2} ± {:.2}; max {:.2} {}",
            self.tag, stats.average(), stats.stdev(), stats.max(), stats.unit_postfix(),
        )
    }
}
