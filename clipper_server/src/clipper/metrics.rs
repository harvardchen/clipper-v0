use time;
use time::{Timespec};
use std::cell::{RefCell};
use std::sync::atomic::{AtomicUsize, AtomicIsize, Ordering};
use rand::{thread_rng, Rng};
use std::sync::{RwLock, Arc};
use tsdb::{Tsdb};

const NUM_MICROS_PER_SEC: i64 = 1_000_000;
const NUM_NANOS_PER_SEC: i64 = 1_000_000_000;
const NUM_NANOS_PER_MICRO: i64 = 1_000;
const SECONDS_PER_MINUTE: f64 = 60.0;
const FIVE_SECONDS: f64 = 5.0;
const ONE_MINUTE: u16 = 1;
const FIVE_MINUTES: u16 = 5;
const FIFTEEN_MINUTES: u16 = 15;

trait Metric {

    fn report(&self) -> String;

    /// Must have way to atomically clear state
    fn clear(&self);

}


/// Counts a value
pub struct Counter {
    pub name: String,
    count: AtomicIsize,
}

#[derive(Debug)]
struct CounterStats {
    name: String,
    count: isize,
}

impl Metric for Counter {
    fn clear(&self) {
        self.count.store(0, Ordering::SeqCst);
    }

    fn report(&self) -> String {
        let stats = CounterStats {
            name: self.name.clone(),
            count: self.count.load(Ordering::SeqCst),
        };
        format!("{:?}", stats)
    }
}

impl Counter {
    pub fn new(name: String, start_count: isize) -> Counter {
        Counter {
            name: name,
            count: AtomicIsize::new(start_count),
        }
    }

    pub fn incr(&self, increment: isize) {
        self.count.fetch_add(increment, Ordering::Relaxed);
    }

    pub fn decr(&self, decrement: isize) {
        self.count.fetch_sub(decrement, Ordering::Relaxed);
    }

    pub fn value(&self) -> isize {
        self.count.load(Ordering::SeqCst)
    }
}

pub struct RatioCounter {
    pub name: String,
    numerator: AtomicUsize,
    denominator: AtomicUsize,
}

#[derive(Debug)]
struct RatioStats {
    name: String,
    ratio: f64,
}

impl Metric for RatioCounter {
    // TODO: This has a race condition.
    fn clear(&self) {
        self.denominator.store(0, Ordering::SeqCst);
        self.numerator.store(0, Ordering::SeqCst);
    }

    fn report(&self) -> String {
        let ratio = self.numerator.load(Ordering::SeqCst) as f64 /
                    self.denominator.load(Ordering::SeqCst) as f64;
        let stats = RatioStats {
            name: self.name.clone(),
            ratio: ratio,
        };
        format!("{:?}", stats)
    }
}

impl RatioCounter {
    pub fn new(name: String, n: usize, d: usize) -> RatioCounter {
        RatioCounter {
            name: name,
            numerator: AtomicUsize::new(n),
            denominator: AtomicUsize::new(d),
        }
    }

    pub fn incr(&self, n_incr: usize, d_incr: usize) {
        self.numerator.fetch_add(n_incr, Ordering::Relaxed);
        self.denominator.fetch_add(d_incr, Ordering::Relaxed);
    }

    pub fn get_ratio(&self) -> f64 {
        let n = self.numerator.load(Ordering::Relaxed);
        let d = self.denominator.load(Ordering::Relaxed);
        if d == 0 {
            warn!("{} ratio denominator is 0", self.name);
            ::std::f64::NAN
        } else {
            n as f64 / d as f64
        }
    }
}

/// Represents the different load average options for exponentially
/// weighted moving averages (EWMA) within meters. For a one minute EWMA,
/// we use the OneMinute load average, etc...
enum LoadAverage {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
}

/// Represents an exponentially weighted moving average.
/// Multiple EWMAs are included within a single meter
/// corresponding to different load averages.
struct EWMA {
    uncounted: AtomicUsize,
    alpha: f64,
    interval: f64,
    rate: RwLock<f64>,
}

impl EWMA {
    /// Constructs a new exponentially weighted moving average.
    ///
    /// * `interval` - The tick interval in seconds for the EWMA (how frequently updates are expected)
    /// * `load_avg` - The load average for the EWMA (see LoadAverage enum for details)
    fn new(interval: f64, load_avg: LoadAverage) -> EWMA {
        let alpha_exp = match load_avg {
            LoadAverage::OneMinute => (-interval / (SECONDS_PER_MINUTE as f64) / (ONE_MINUTE as f64)).exp(),
            LoadAverage::FiveMinutes => (-interval / (SECONDS_PER_MINUTE as f64) / (FIVE_MINUTES as f64)).exp(),
            LoadAverage::FifteenMinutes => (-interval / (SECONDS_PER_MINUTE as f64) / (FIFTEEN_MINUTES as f64)).exp(),
        };

        EWMA {
            uncounted: AtomicUsize::new(0),
            alpha: 1.0 - alpha_exp,
            interval: interval,
            rate: RwLock::new(-1.0),
        }
    }

    /// Updates the rate of the EWMA by incoporating unaccounted data 
    /// and decaying based on the calculated "alpha" factor.
    fn tick(&self) {
        match self.rate.write() {
            Ok(mut rate) => {
                let count = self.uncounted.swap(0, Ordering::Relaxed);
                let current_rate = (count as f64) / self.interval;
                if *rate == -1.0 {
                    // If our rate is -1, we have never calculated a valid rate before, so
                    // set the rate to the be the number of events over the first interval
                    *rate = current_rate;
                } else {
                    *rate += self.alpha * (current_rate - *rate);
                }
            },
            Err(_) => {},
        }
    }

    fn mark(&self, num: usize) {
        self.uncounted.fetch_add(num, Ordering::Relaxed);
    }

    fn get_rate_secs(&self) -> f64 {
        *self.rate.read().unwrap()
    }
}

pub trait Clock {
    fn get_time(&self) -> Timespec;
}

pub struct RealtimeClock {

}

impl RealtimeClock {
    fn new() -> RealtimeClock {
        RealtimeClock {}
    }
}

impl Clock for RealtimeClock {
    fn get_time(&self) -> Timespec {
        time::get_time()
    } 
}

pub struct ManualClock {
    time: RefCell<Vec<Timespec>>,
}

impl ManualClock {
    pub fn new(times: Vec<Timespec>) -> ManualClock {
        ManualClock {
            time: RefCell::new(times),
        }
    }
}

impl Clock for ManualClock {
    fn get_time(&self) -> Timespec {
        self.time.borrow_mut().pop().unwrap()
    }
}


/// Measures the rate of an event occurring.
pub struct Meter<C> where C: Clock {
    pub name: String,
    pub unit: String,
    pub clock: C,
    start_time: RwLock<Timespec>,
    tick_interval: f64,
    last_tick: AtomicUsize,
    count: AtomicUsize,
    m1rate: EWMA,
    m5rate: EWMA,
    m15rate: EWMA,
}

impl <C> Meter<C> where C: Clock {
    pub fn new(name: String, clock: C) -> Meter<C> {
        let curr_time = clock.get_time();
        let curr_nanos = (curr_time.sec * NUM_NANOS_PER_SEC) + curr_time.nsec as i64;
        let tick_interval = FIVE_SECONDS as f64;
        Meter {
            name: name,
            unit: "events per second".to_string(),
            clock: clock,
            start_time: RwLock::new(curr_time),
            last_tick: AtomicUsize::new(curr_nanos as usize),
            tick_interval: tick_interval,
            count: AtomicUsize::new(0),
            m1rate: EWMA::new(tick_interval, LoadAverage::OneMinute),
            m5rate: EWMA::new(tick_interval, LoadAverage::FiveMinutes),
            m15rate: EWMA::new(tick_interval, LoadAverage::FifteenMinutes),
        }
    }

    pub fn mark(&self, num: usize) {
        self.count.fetch_add(num, Ordering::Relaxed);
        self.m1rate.mark(num);
        self.m5rate.mark(num);
        self.m15rate.mark(num);
    }

    fn get_rate_micros(&self) -> f64 {
        let cur_time = self.clock.get_time();
        let cur_micros = (cur_time.sec * NUM_MICROS_PER_SEC) + (cur_time.nsec as i64 / NUM_NANOS_PER_MICRO);
        let start_time = self.start_time.read().unwrap();
        let start_micros = (start_time.sec * NUM_MICROS_PER_SEC) + (start_time.nsec as i64 / NUM_NANOS_PER_MICRO);
        let count = self.count.load(Ordering::SeqCst);
        let rate = count as f64 / (cur_micros - start_micros) as f64;
        rate
    }

    fn tick_if_necessary(&self) {
        let curr_time = self.clock.get_time();
        let curr_nanos = (curr_time.sec * NUM_NANOS_PER_SEC) + curr_time.nsec as i64;
        let last_tick_nanos = self.last_tick.load(Ordering::SeqCst);
        let time_since_tick = curr_nanos - last_tick_nanos as i64;
        let tick_interval_nanos = self.tick_interval as i64 * NUM_NANOS_PER_SEC;

        if time_since_tick > tick_interval_nanos {
            let new_last_tick = curr_nanos - (time_since_tick % tick_interval_nanos);
            if self.last_tick.compare_and_swap(last_tick_nanos, new_last_tick as usize, Ordering::SeqCst) == last_tick_nanos {
                let num_ticks = time_since_tick / tick_interval_nanos;
                for _ in 0..num_ticks {
                    self.m1rate.tick();
                    self.m5rate.tick();
                    self.m15rate.tick();
                }
            }
        }
    }

    /// Returns the rate of this meter in events
    /// per second for the last minute, based on an
    /// exponentially weighted moving average
    pub fn get_one_minute_rate_secs(&self) -> f64 {
        self.tick_if_necessary();
        self.m1rate.get_rate_secs()
    }

    /// Returns the rate of this meter in events
    /// per second for the last five minutes, based on an
    /// exponentially weighted moving average
    pub fn get_five_minute_rate_secs(&self) -> f64 {
        self.tick_if_necessary();
        self.m5rate.get_rate_secs()
    }   

    /// Returns the rate of this meter in events
    /// per second for the fifteen minutes, based on an
    /// exponentially weighted moving average
    pub fn get_fifteen_minute_rate_secs(&self) -> f64 {
        self.tick_if_necessary();
        self.m15rate.get_rate_secs()
    }

    /// Returns the rate of this meter in
    /// events per second since the time of initialization.
    pub fn get_rate_secs(&self) -> f64 {
        self.get_rate_micros() * NUM_MICROS_PER_SEC as f64
    }
}

#[derive(Debug)]
struct MeterStats {
    name: String,
    rate: f64,
    one_min_rate: f64,
    five_min_rate: f64,
    fifteen_min_rate: f64,
    unit: String,
}

impl <C> Metric for Meter<C> where C: Clock {
    fn clear(&self) {
        let mut t = self.start_time.write().unwrap();
        *t = self.clock.get_time();
        self.count.store(0, Ordering::SeqCst);
    }

    fn report(&self) -> String {

        let stats = MeterStats {
            name: self.name.clone(),
            rate: self.get_rate_secs(),
            one_min_rate: self.get_one_minute_rate_secs(),
            five_min_rate: self.get_five_minute_rate_secs(),
            fifteen_min_rate: self.get_fifteen_minute_rate_secs(),
            unit: self.unit.clone(),
        };
        format!("{:?}", stats)
    }
}

#[derive(Debug)]
pub struct HistStats {
    pub name: String,
    pub size: u64,
    pub min: i64,
    pub max: i64,
    pub mean: f64,
    pub std: f64,
    pub p95: f64,
    pub p99: f64,
    pub p50: f64,
}

// This gives me latency distribution, min, mean, max, etc.
pub struct Histogram {
    pub name: String,
    sample: RwLock<ReservoirSampler>,
}

impl Histogram {
    pub fn new(name: String, sample_size: usize) -> Histogram {
        Histogram {
            name: name,
            sample: RwLock::new(ReservoirSampler::new(sample_size)),
        }
    }

    pub fn insert(&self, item: i64) {
        let mut res = self.sample.write().unwrap();
        res.sample(item);
    }

    pub fn stats(&self) -> HistStats {
        let mut snapshot: Vec<i64> = {
            self.sample.read().unwrap().snapshot()
        };
        let sample_size = snapshot.len();
        if sample_size == 0 {
            HistStats {
                name: self.name.clone(),
                size: 0,
                min: 0,
                max: 0,
                mean: 0.0,
                std: 0.0,
                p95: 0.0,
                p99: 0.0,
                p50: 0.0,
            }

        } else {
            snapshot.sort();
            let min = snapshot.first().unwrap();
            let max = snapshot.last().unwrap();
            let p99 = Histogram::percentile(&snapshot, 0.99);
            let p95 = Histogram::percentile(&snapshot, 0.95);
            let p50 = Histogram::percentile(&snapshot, 0.50);
            let mean = snapshot.iter().fold(0, |acc, &x| acc + x) as f64 / snapshot.len() as f64;
            let var: f64 = if sample_size > 1 {
                snapshot.iter().fold(0.0, |acc, &x| acc + (x as f64 - mean).powi(2)) / (sample_size - 1) as f64
            } else {
                0.0
            };
            HistStats {
                name: self.name.clone(),
                size: sample_size as u64,
                min: *min,
                max: *max,
                mean: mean,
                std: var.sqrt(),
                p95: p95,
                p99: p99,
                p50: p50,
            }
        }
    }


    /// Compute the percentile rank of `snapshot`. The rank `p` must
    /// be in [0.0, 1.0] (inclusive) and `snapshot` must be sorted.
    ///
    /// Algorithm is the third variant from
    /// [Wikipedia](https://en.wikipedia.org/wiki/Percentile)
    pub fn percentile(snapshot: &Vec<i64>, p: f64) -> f64 {
        assert!(snapshot.len() > 0);
        let sample_size = snapshot.len() as f64;
        assert!(p >= 0.0 && p <= 1.0, "percentile out of bounds");
        let x = if p <= 1.0 / (sample_size + 1.0) {
            // println!("a");
            1.0
        } else if p > 1.0 / (sample_size + 1.0) && p < sample_size / (sample_size + 1.0) {
            // println!("b");
            p * (sample_size + 1.0)
        } else {
            // println!("c");
            sample_size
        };
        let index = x.floor() as usize - 1;
        let v = snapshot[index] as f64;
        let rem = x % 1.0;
        let per = if rem != 0.0 {
            // println!("rem: {}", rem);
            v + rem * (snapshot[index + 1] - snapshot[index]) as f64
        } else {
            v
        };
        per
    }

    // // TODO: this percentile calculation is wrong for numbers less than 100
    // fn percentile(snapshot: &Vec<i64>, p: usize) -> f64 {
    //     assert!(p <= 100, "must supply a percentile between 0 and 100");
    //     let sample_size = snapshot.len();
    //     let per = if sample_size == 1 {
    //         snapshot[0]
    //     } else if sample_size < 100 {
    //         warn!("computing p{} of sample size smaller than 100", p);
    //         snapshot[(sample_size - 1 - (100 - p)) as usize] as f64
    //     } else if (sample_size % 100) == 0 {
    //         let per_index: usize = sample_size * p / 100;
    //         snapshot[per_index as usize] as f64
    //     } else {
    //         let per_index: f64 = (sample_size as f64) * (p as f64 / 100.0);
    //         let per_below = per_index.floor() as usize;
    //         let per_above = per_index.ceil() as usize;
    //         (snapshot[per_below] as f64 + snapshot[per_above] as f64) / 2.0_f64
    //     };
    //     per
    // }
}


#[cfg(test)]
#[cfg_attr(rustfmt, rustfmt_skip)]
mod tests {
    use super::*;
    use time::Timespec;

    #[test]
    fn percentile() {
        let snap = vec![15, 20, 35, 40, 50];
        let p = 0.4;
        let computed_percentile = Histogram::percentile(&snap, p);
        assert!(computed_percentile - 26.0 < 0.000001);
    }

    #[test]
    fn percentile_one_elem() {
        let snap = vec![15];
        let p = 0.4;
        let computed_percentile = Histogram::percentile(&snap, p);
        assert!(computed_percentile - 15.0 < 0.000001);
    }

    #[test]
    fn percentile_one_elem_pzero() {
        let snap = vec![15];
        let p = 0.0;
        let computed_percentile = Histogram::percentile(&snap, p);
        assert!(computed_percentile - 15.0 < 0.000001);
    }

    #[test]
    fn percentile_one_elem_p100() {
        let snap = vec![15];
        let p = 1.0;
        let computed_percentile = Histogram::percentile(&snap, p);
        assert!(computed_percentile - 15.0 < 0.000001);
    }

    #[test]
    #[should_panic]
    fn percentile_zero_elem() {
        let snap = Vec::new();
        let p = 0.5;
        let _ = Histogram::percentile(&snap, p);
    }

    #[test]
    fn meter_test() {
        let mut times: Vec<Timespec> = Vec::new();
        times.push(Timespec::new(5,0));
        times.push(Timespec::new(0,0));
        let clock = ManualClock::new(times.to_owned());
        let meter = Meter::new("test".to_string(), clock);
        meter.mark(1);
        let rate = meter.get_one_minute_rate_secs();
        info!("RATE: {}", rate);
    }
}



impl Metric for Histogram {
    fn clear(&self) {
        let mut res = self.sample.write().unwrap();
        res.clear();
    }

    fn report(&self) -> String {
        format!("{:?}", self.stats())
    }
}



struct ReservoirSampler {
    reservoir: Vec<i64>,
    sample_size: usize,
    n: usize,
}

impl ReservoirSampler {
    pub fn new(sample_size: usize) -> ReservoirSampler {
        ReservoirSampler {
            reservoir: Vec::with_capacity(sample_size),
            sample_size: sample_size,
            n: 0,
        }
    }

    pub fn sample(&mut self, value: i64) {
        if self.n < self.sample_size {
            self.reservoir.push(value);
        } else {
            assert!(self.reservoir.len() == self.sample_size);
            let mut rng = thread_rng();
            let j = rng.gen_range(0, self.n + 1); // exclusive
            if j < self.sample_size {
                self.reservoir[j] = value;
            }
        }
        self.n += 1;
    }

    pub fn clear(&mut self) {
        self.reservoir.clear();
        self.n = 0;
    }

    pub fn snapshot(&self) -> Vec<i64> {
        self.reservoir.clone()
    }
}


pub struct Registry {
    pub name: String,
    db: Tsdb,
    counters: Vec<Arc<Counter>>,
    // sum_counters: Vec<SumCounter>,
    ratio_counters: Vec<Arc<RatioCounter>>,
    histograms: Vec<Arc<Histogram>>,
    meters: Vec<Arc<Meter<RealtimeClock>>>,
}

impl Registry {
    pub fn new(name: String, db_ip: String, db_port: u16) -> Registry {
        // Create a new time series database to store these metrics
        Registry {
            name: name.clone(),
            db: Tsdb::new(name.clone(), db_ip.to_string(), db_port),
            counters: Vec::new(),
            ratio_counters: Vec::new(),
            histograms: Vec::new(),
            meters: Vec::new(),
        }
    }

    pub fn create_histogram(&mut self, name: String, sample_size: usize) -> Arc<Histogram> {
        let hist = Arc::new(Histogram::new(name, sample_size));
        self.histograms.push(hist.clone());
        hist
    }

    pub fn create_meter(&mut self, name: String) -> Arc<Meter<RealtimeClock>> {
        let meter = Arc::new(Meter::new(name,RealtimeClock::new()));
        self.meters.push(meter.clone());
        meter
    }

    pub fn create_ratio_counter(&mut self, name: String) -> Arc<RatioCounter> {
        let counter = Arc::new(RatioCounter::new(name, 0, 0));
        self.ratio_counters.push(counter.clone());
        counter
    }

    pub fn create_counter(&mut self, name: String) -> Arc<Counter> {
        let counter = Arc::new(Counter::new(name, 0));
        self.counters.push(counter.clone());
        counter
    }

    pub fn report(&self) -> String {
        let mut report_string = String::new();
        report_string.push_str(&format!("\n{} Metrics\n", self.name));

        if self.counters.len() > 0 {
            report_string.push_str("\tCounters:\n");
            for x in self.counters.iter() {
                report_string.push_str(&format!("\t\t{}\n", x.report()));
            }
        }

        if self.ratio_counters.len() > 0 {
            report_string.push_str("\tRatios:\n");
            for x in self.ratio_counters.iter() {
                report_string.push_str(&format!("\t\t{}\n", x.report()));
            }
        }

        if self.histograms.len() > 0 {
            report_string.push_str("\tHistograms:\n");
            for x in self.histograms.iter() {
                report_string.push_str(&format!("\t\t{}\n", x.report()));
            }
        }

        if self.meters.len() > 0 {
            report_string.push_str("\tMeters:\n");
            for x in self.meters.iter() {
                report_string.push_str(&format!("\t\t{}\n", x.report()));
            }
        }


        debug!("{}", report_string);
        report_string
    }

    pub fn persist(&self) {
        let mut write = self.db.new_write();
        for x in self.counters.iter() {
            write.append_counter(x);
        }
        for x in self.meters.iter() {
            write.append_meter(x);
        }
        for x in self.ratio_counters.iter() {
            write.append_ratio(x);
        }
        for x in self.histograms.iter() {
            write.append_histogram(x);
        }
        write.execute();
    }

    pub fn reset(&self) {
        for x in self.counters.iter() {
            x.clear();
        }
        for x in self.ratio_counters.iter() {
            x.clear();
        }

        for x in self.histograms.iter() {
            x.clear();
        }

        for x in self.meters.iter() {
            x.clear();
        }
    }
}
