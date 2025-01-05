//! Simple benchmarking functionality for supporting thyme.
//!
//! Benchmarks consist of a moving average and associated statistics of a given
//! set of timings.  Timings that are grouped together share the same tag.
//! You can pass a block to be timed using [`run`](fn.run.html), or create a handle with
//! [`start`](fn.start.html) and end the timing with [`end`](struct.Handle.html#method.end).
//! Use [`stats`](fn.stats.html) to get a [`Stats`](struct.Stats.html), which is the
//! primary interface for reporting on the timings.

use std::time::{Duration, Instant};

use parking_lot::{const_mutex, Mutex};

const MOVING_AVG_LEN: usize = 30;

static BENCH: Mutex<BenchSet> = const_mutex(BenchSet::new());

/// Configuration values to pass to the benchmark [`report`](fn.report.html) function.
#[derive(Copy, Clone)]
pub struct ReportConfig {
    /// The length of the given report
    pub length: ReportConfigLength,

    /// The number of samples to average for the given report
    pub samples: ReportConfigSamples,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportConfig {
    /// Constructs a default Report config with full length and all samples
    pub fn new() -> Self {
        Self {
            length: ReportConfigLength::Long,
            samples: ReportConfigSamples::All,
        }
    }

    /// Builds a report config utilizing the specified `length`
    pub fn with_length(self, length: ReportConfigLength) -> Self {
        Self {
            length,
            samples: self.samples,
        }
    }

    /// Builds a report config utilizing the specified `samples`
    pub fn with_samples(self, samples: ReportConfigSamples) -> Self {
        Self {
            length: self.length,
            samples,
        }
    }

    /// Builds a report with short report length
    pub fn with_short_length(self) -> Self {
        Self {
            length: ReportConfigLength::Short,
            samples: self.samples,
        }
    }

    /// Builds a report with long report length.
    pub fn with_long_length(self) -> Self {
        Self {
            length: ReportConfigLength::Long,
            samples: self.samples,
        }
    }

    /// Builds a report using all samples
    pub fn with_all_samples(self) -> Self {
        Self {
            length: self.length,
            samples: ReportConfigSamples::All
        }
    }

    /// Builds a report using a moving average of samples
    pub fn with_moving_average_samples(self) -> Self {
        Self {
            length: self.length,
            samples: ReportConfigSamples::MovingAverage,
        }
    }
}

/// For a given benchmark report, whether to use the condensed report output
#[derive(Copy, Clone)]
pub enum ReportConfigLength {
    /// Use the condensed report output
    Short,

    /// Use the full length report output
    Long,
}

/// For a given benchmark report, how many samples to average
#[derive(Copy, Clone)]
pub enum ReportConfigSamples {
    /// Average all samples that have ever been taken
    All,

    /// Perform a moving average of the most recent samples taken
    MovingAverage,
}

impl ReportConfigSamples {
    fn limit(&self) -> Option<usize> {
        match self {
            ReportConfigSamples::All => None,
            ReportConfigSamples::MovingAverage => Some(MOVING_AVG_LEN),
        }
    }
}

/// A benchmarking handle created by [`start`](fn.start.html).  [`end`](#method.end) this to
/// finish the given benchmark timing
pub struct Handle {
    index: usize
}

impl Handle {
    /// Finish the timing associated with this handle.
    pub fn end(self) {
        end(self);
    }
}

/// Runs the specified closure `block` as a benchmark timing
/// with the given `tag`.
pub fn run<Ret, F: FnOnce() -> Ret>(tag: &str, block: F) -> Ret {
    let handle = start(tag);
    let ret = (block)();
    end(handle);
    ret
}

/// Starts a benchmark timing with the given `tag`.  You
/// must [`end`](struct.Handle.html#method.end) the returned [`Handle`](struct.Handle.html) to complete
/// the timing.
#[must_use]
pub fn start(tag: &str) -> Handle {
    let mut bench = BENCH.lock();
    bench.start(tag)
}

/// Returns a `Stats` object for the benchmark timings
/// associated with the given `tag`, utilizing the specified
/// `samples`.
pub fn stats(tag: &str, samples: ReportConfigSamples) -> Stats {
    let bench = BENCH.lock();

    bench.stats(tag, samples.limit())
}

/// Returns stats for all benchmark tags that have been benchmarked.
/// See [`stats`](fn.stats.html).
pub fn stats_all(samples: ReportConfigSamples) -> Vec<Stats> {
    let bench = BENCH.lock();
    bench.stats_all(samples.limit())
}

/// Generate a report string for all tags that have been
/// benchmarked.  See [`report`](fn.report.html)
pub fn report_all(config: ReportConfig) -> Vec<String> {
    let bench = BENCH.lock();
    let limit = config.samples.limit();

    match config.length {
        ReportConfigLength::Long => bench.report_all(limit),
        ReportConfigLength::Short => bench.short_report_all(limit),
    }
}

/// Generate a report string for the given `tag`, utilizing the
/// specified report `config`.  The report will include the
/// data in the [`Stats`](struct.Stats.html) associated with this `tag`,
/// and be formatted with the appropriate units.
pub fn report(tag: &str, config: ReportConfig) -> String {
    let bench = BENCH.lock();
    let limit = config.samples.limit();

    match config.length {
        ReportConfigLength::Long => bench.report(tag, limit),
        ReportConfigLength::Short => bench.short_report(tag, limit),
    }
}

fn end(handle: Handle) {
    let mut bench = BENCH.lock();
    bench.end(handle);
}

/// Statistics associated with a given set of benchmark timings.
/// These are obtained with the `stats` method for a given tag.
/// Statistics are for a moving average of the last N timings for the
/// tag, where N is currently hardcoded to 30.
#[derive(Debug, Copy, Clone)]
pub struct Stats {
    count: usize,
    total_s: f32,
    average_s: f32,
    stdev_s: f32,
    max_s: f32,
    unit: Unit,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            count: 0,
            total_s: 0.0,
            average_s: 0.0,
            stdev_s: 0.0,
            max_s: 0.0,
            unit: Unit::Seconds,
        }
    }
}

impl Stats {
    /// Returns the sum total of the timings, in the current unit
    /// of this `Stats`.
    pub fn total(&self) -> f32 {
        self.total_s * self.unit.multiplier()
    }

    /// Returns the average of the timings, in the current unit
    /// of this `Stats`.
    pub fn average(&self) -> f32 {
        self.average_s * self.unit.multiplier()
    }

    /// Returns the standard devication of the timings, in the current unit
    /// of this `Stats`.
    pub fn stdev(&self) -> f32 {
        self.stdev_s * self.unit.multiplier()
    }

    /// Returns the maximum of the timings, in the current unit
    /// of this `Stats`.
    pub fn max(&self) -> f32 {
        self.max_s * self.unit.multiplier()
    }

    /// Returns the postfix string of the Unit associated with this
    /// `Stats`, such as "s" for Seconds, "ms" for milliseconds, and
    /// "µs" for microseconds.
    pub fn unit_postfix(&self) -> &'static str {
        self.unit.postfix()
    }

    /// Automatically picks an appropriate unit for this `Stats` based
    /// on the size of the average value, and converts the stats to
    /// use that unit.
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

    /// Converts this `Stats` to use seconds as a unit
    pub fn in_seconds(mut self) -> Stats {
        self.unit = Unit::Seconds;
        self
    }

    /// Converts this `Stats` to use milliseconds as a unit
    pub fn in_millis(mut self) -> Stats {
        self.unit = Unit::Millis;
        self
    }

    /// Converts this `Stats` to use microseconds as a unit
    pub fn in_micros(mut self) -> Stats {
        self.unit = Unit::Micros;
        self
    }
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

    fn stats(&self, tag: &str, limit: Option<usize>) -> Stats {
        for bench in self.benches.iter() {
            if bench.tag == tag {
                return bench.stats(limit);
            }
        }

        Stats::default()
    }

    fn stats_all(&self, limit: Option<usize>) -> Vec<Stats> {
        let mut out = Vec::new();
        for bench in self.benches.iter() {
            out.push(bench.stats(limit));
        }
        out
    }

    fn report(&self, tag: &str, limit: Option<usize>) -> String {
        for bench in self.benches.iter() {
            if bench.tag == tag {
                return bench.report_str(limit);
            }
        }

        "Bench not found".to_string()
    }

    fn short_report(&self, tag: &str, limit: Option<usize>) -> String {
        for bench in self.benches.iter() {
            if bench.tag == tag {
                return bench.short_report_str(limit);
            }
        }

        "Bench not found".to_string()
    }

    fn short_report_all(&self, limit: Option<usize>) -> Vec<String> {
        let mut out = Vec::new();
        for bench in self.benches.iter() {
            out.push(bench.short_report_str(limit));
        }
        out
    }

    fn report_all(&self, limit: Option<usize>) -> Vec<String> {
        let mut out = Vec::new();
        for bench in self.benches.iter() {
            out.push(bench.report_str(limit));
        }
        out
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

impl Bench {
    fn new(tag: String) -> Bench {
        Bench {
            history: Vec::new(),
            start: None,
            tag,
        }
    }

    fn stats(&self, limit: Option<usize>) -> Stats {
        let len = self.history.len();
        let count = match limit {
            None => len,
            Some(limit) => std::cmp::min(len, limit),
        };

        let data = || { self.history.iter().rev().take(count) };

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
            count,
            total_s: sum,
            average_s: avg,
            stdev_s: stdev,
            max_s: max,
            unit: Unit::Seconds,
        }
    }

    fn short_report_str(&self, limit: Option<usize>) -> String {
        let stats = self.stats(limit).pick_unit();
        if self.history.len() == 1 {
            format!("{}: {:.2} {}", self.tag, stats.average(), stats.unit_postfix())
        } else {
            format!(
                "{}: {:.2} ± {:.2} {}",
                self.tag, stats.average(), stats.stdev(), stats.unit_postfix(),
            )
        }
    }

    fn report_str(&self, limit: Option<usize>) -> String {
        let stats = self.stats(limit).pick_unit();
        if self.history.len() == 1 {
            format!("{}: {:.2} {}", self.tag, stats.average(), stats.unit_postfix())
        } else {
            format!(
                "{} ({} Samples): {:.2} ± {:.2}; max {:.2}, total {:.2} {}",
                self.tag, stats.count, stats.average(), stats.stdev(), stats.max(), stats.total(), stats.unit_postfix(),
            )
        }
    }
}
