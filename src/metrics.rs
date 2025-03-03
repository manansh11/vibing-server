use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// A simple counter that can be incremented atomically
#[derive(Debug)]
pub struct Counter {
    value: AtomicUsize,
}

impl Counter {
    /// Create a new counter with an initial value
    pub fn new(initial_value: usize) -> Self {
        Self {
            value: AtomicUsize::new(initial_value),
        }
    }
    
    /// Increment the counter by a specific amount
    pub fn increment(&self, amount: usize) {
        self.value.fetch_add(amount, Ordering::Relaxed);
    }
    
    /// Get the current value of the counter
    pub fn value(&self) -> usize {
        self.value.load(Ordering::Relaxed)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A histogram for tracking distribution of values
#[derive(Debug)]
pub struct Histogram {
    buckets: RwLock<Vec<(f64, AtomicUsize)>>,
    count: AtomicUsize,
    sum: AtomicUsize,
    min: AtomicUsize,
    max: AtomicUsize,
}

impl Histogram {
    /// Create a new histogram with specified buckets
    pub fn new(bucket_boundaries: &[f64]) -> Self {
        let mut buckets = Vec::with_capacity(bucket_boundaries.len());
        
        for &boundary in bucket_boundaries {
            buckets.push((boundary, AtomicUsize::new(0)));
        }
        
        Self {
            buckets: RwLock::new(buckets),
            count: AtomicUsize::new(0),
            sum: AtomicUsize::new(0),
            min: AtomicUsize::new(usize::MAX),
            max: AtomicUsize::new(0),
        }
    }
    
    /// Create a histogram with exponential buckets
    pub fn exponential(start: f64, factor: f64, count: usize) -> Self {
        let mut boundaries = Vec::with_capacity(count);
        let mut current = start;
        
        for _ in 0..count {
            boundaries.push(current);
            current *= factor;
        }
        
        Self::new(&boundaries)
    }
    
    /// Record a value in the histogram
    pub fn record(&self, value: f64) {
        // Update basic statistics
        let value_as_usize = value as usize;
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value_as_usize, Ordering::Relaxed);
        
        // Update min/max values
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value_as_usize < current_min {
            match self.min.compare_exchange_weak(
                current_min,
                value_as_usize,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_min) => current_min = new_min,
            }
        }
        
        let mut current_max = self.max.load(Ordering::Relaxed);
        while value_as_usize > current_max {
            match self.max.compare_exchange_weak(
                current_max,
                value_as_usize,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => current_max = new_max,
            }
        }
        
        // Update bucket counters
        let buckets = self.buckets.read().unwrap();
        for (boundary, counter) in buckets.iter() {
            if value <= *boundary {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// Get the count of values in the histogram
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    
    /// Get the sum of values in the histogram
    pub fn sum(&self) -> usize {
        self.sum.load(Ordering::Relaxed)
    }
    
    /// Get the minimum value recorded
    pub fn min(&self) -> usize {
        self.min.load(Ordering::Relaxed)
    }
    
    /// Get the maximum value recorded
    pub fn max(&self) -> usize {
        self.max.load(Ordering::Relaxed)
    }
    
    /// Get the mean value
    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        
        self.sum() as f64 / count as f64
    }
    
    /// Get the bucket counts
    pub fn buckets(&self) -> Vec<(f64, usize)> {
        let buckets = self.buckets.read().unwrap();
        buckets
            .iter()
            .map(|(boundary, counter)| (*boundary, counter.load(Ordering::Relaxed)))
            .collect()
    }
}

/// A timer for measuring durations
#[derive(Debug)]
pub struct Timer {
    start: Instant,
    histogram: Arc<Histogram>,
}

impl Timer {
    /// Create a new timer
    pub fn new(histogram: Arc<Histogram>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
    
    /// Stop the timer and record the duration
    pub fn stop(&self) {
        let duration = self.start.elapsed();
        self.histogram.record(duration.as_micros() as f64);
    }
    
    /// Get the elapsed time without stopping the timer
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A registry for storing and accessing metrics
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, Arc<Counter>>>,
    histograms: RwLock<HashMap<String, Arc<Histogram>>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }
    
    /// Get or create a counter
    pub fn counter(&self, name: &str) -> Arc<Counter> {
        {
            let counters = self.counters.read().unwrap();
            if let Some(counter) = counters.get(name) {
                return counter.clone();
            }
        }
        
        let mut counters = self.counters.write().unwrap();
        let counter = Arc::new(Counter::default());
        counters.insert(name.to_string(), counter.clone());
        counter
    }
    
    /// Get or create a histogram
    pub fn histogram(&self, name: &str, bucket_boundaries: &[f64]) -> Arc<Histogram> {
        {
            let histograms = self.histograms.read().unwrap();
            if let Some(histogram) = histograms.get(name) {
                return histogram.clone();
            }
        }
        
        let mut histograms = self.histograms.write().unwrap();
        let histogram = Arc::new(Histogram::new(bucket_boundaries));
        histograms.insert(name.to_string(), histogram.clone());
        histogram
    }
    
    /// Get or create a histogram with exponential buckets
    pub fn exponential_histogram(
        &self,
        name: &str,
        start: f64,
        factor: f64,
        count: usize,
    ) -> Arc<Histogram> {
        {
            let histograms = self.histograms.read().unwrap();
            if let Some(histogram) = histograms.get(name) {
                return histogram.clone();
            }
        }
        
        let mut histograms = self.histograms.write().unwrap();
        let histogram = Arc::new(Histogram::exponential(start, factor, count));
        histograms.insert(name.to_string(), histogram.clone());
        histogram
    }
    
    /// Create a timer for measuring operation duration
    pub fn timer(&self, name: &str) -> Timer {
        // Default histogram for timing operations (in microseconds)
        // Buckets from 1us to ~10s
        let histogram = self.exponential_histogram(name, 1.0, 2.0, 24);
        Timer::new(histogram)
    }
    
    /// Get metrics as a formatted string
    pub fn format(&self) -> String {
        let mut result = String::new();
        
        // Format counters
        {
            let counters = self.counters.read().unwrap();
            for (name, counter) in counters.iter() {
                result.push_str(&format!("{}: {}\n", name, counter.value()));
            }
        }
        
        // Format histograms
        {
            let histograms = self.histograms.read().unwrap();
            for (name, histogram) in histograms.iter() {
                result.push_str(&format!(
                    "{}: count={}, sum={}, min={}, max={}, mean={:.2}\n",
                    name,
                    histogram.count(),
                    histogram.sum(),
                    histogram.min(),
                    histogram.max(),
                    histogram.mean()
                ));
                
                result.push_str("  Buckets:\n");
                for (boundary, count) in histogram.buckets() {
                    result.push_str(&format!("    <= {:.2}: {}\n", boundary, count));
                }
            }
        }
        
        result
    }
}

/// The metrics collector for the server
pub struct MetricsCollector {
    registry: Arc<MetricsRegistry>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            registry: Arc::new(MetricsRegistry::new()),
        }
    }
    
    /// Get the metrics registry
    pub fn registry(&self) -> Arc<MetricsRegistry> {
        self.registry.clone()
    }
    
    /// Record a connection event
    pub fn record_connection(&self, event_type: &str) {
        let counter = self.registry.counter(&format!("connections.{}", event_type));
        counter.increment(1);
    }
    
    /// Record a request event
    pub fn record_request(&self, method: &str, status: u16) {
        let counter = self.registry.counter(&format!("requests.{}.{}", method, status));
        counter.increment(1);
    }
    
    /// Time a request
    pub fn time_request(&self, method: &str) -> Timer {
        self.registry.timer(&format!("request_time.{}", method))
    }
    
    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: usize) {
        let counter = self.registry.counter("bytes_received");
        counter.increment(bytes);
    }
    
    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: usize) {
        let counter = self.registry.counter("bytes_sent");
        counter.increment(bytes);
    }
    
    /// Get a formatted string of all metrics
    pub fn format(&self) -> String {
        self.registry.format()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}