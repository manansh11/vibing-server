use high_performance_server::metrics::{Counter, Histogram, MetricsCollector, MetricsRegistry, Timer};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_counter() {
    let counter = Counter::new(0);
    
    counter.increment(1);
    assert_eq!(counter.value(), 1);
    
    counter.increment(4);
    assert_eq!(counter.value(), 5);
    
    counter.increment(0);
    assert_eq!(counter.value(), 5);
}

#[test]
fn test_counter_concurrent() {
    let counter = Arc::new(Counter::new(0));
    let num_threads = 10;
    let increments_per_thread = 1000;
    
    let mut handles = Vec::with_capacity(num_threads);
    
    for _ in 0..num_threads {
        let counter_clone = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..increments_per_thread {
                counter_clone.increment(1);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(counter.value(), num_threads * increments_per_thread);
}

#[test]
fn test_histogram() {
    let boundaries = [10.0, 20.0, 50.0, 100.0, 200.0];
    let histogram = Histogram::new(&boundaries);
    
    // Record some values
    histogram.record(5.0);
    histogram.record(15.0);
    histogram.record(25.0);
    histogram.record(75.0);
    histogram.record(150.0);
    histogram.record(300.0);
    
    // Check basic stats
    assert_eq!(histogram.count(), 6);
    assert_eq!(histogram.sum(), 570);
    assert_eq!(histogram.min(), 5);
    assert_eq!(histogram.max(), 300);
    assert!((histogram.mean() - 95.0).abs() < 0.01);
    
    // Check bucket counts
    let buckets = histogram.buckets();
    assert_eq!(buckets.len(), boundaries.len());
    
    // <= 10.0: 1 value (5.0)
    assert_eq!(buckets[0].1, 1);
    
    // <= 20.0: 2 values (5.0, 15.0)
    assert_eq!(buckets[1].1, 2);
    
    // <= 50.0: 3 values (5.0, 15.0, 25.0)
    assert_eq!(buckets[2].1, 3);
    
    // <= 100.0: 4 values (5.0, 15.0, 25.0, 75.0)
    assert_eq!(buckets[3].1, 4);
    
    // <= 200.0: 5 values (5.0, 15.0, 25.0, 75.0, 150.0)
    assert_eq!(buckets[4].1, 5);
}

#[test]
fn test_exponential_histogram() {
    let histogram = Histogram::exponential(1.0, 2.0, 5);
    
    // Boundaries should be [1, 2, 4, 8, 16]
    let buckets = histogram.buckets();
    assert_eq!(buckets.len(), 5);
    assert_eq!(buckets[0].0, 1.0);
    assert_eq!(buckets[1].0, 2.0);
    assert_eq!(buckets[2].0, 4.0);
    assert_eq!(buckets[3].0, 8.0);
    assert_eq!(buckets[4].0, 16.0);
    
    // Record some values
    histogram.record(0.5);
    histogram.record(1.5);
    histogram.record(3.0);
    histogram.record(6.0);
    histogram.record(10.0);
    histogram.record(20.0);
    
    // Check bucket counts
    let buckets = histogram.buckets();
    
    // <= 1.0: 1 value (0.5)
    assert_eq!(buckets[0].1, 1);
    
    // <= 2.0: 2 values (0.5, 1.5)
    assert_eq!(buckets[1].1, 2);
    
    // <= 4.0: 3 values (0.5, 1.5, 3.0)
    assert_eq!(buckets[2].1, 3);
    
    // <= 8.0: 4 values (0.5, 1.5, 3.0, 6.0)
    assert_eq!(buckets[3].1, 4);
    
    // <= 16.0: 5 values (0.5, 1.5, 3.0, 6.0, 10.0)
    assert_eq!(buckets[4].1, 5);
}

#[test]
fn test_timer() {
    let histogram = Arc::new(Histogram::exponential(1.0, 10.0, 3));
    let timer = Timer::new(histogram.clone());
    
    thread::sleep(Duration::from_millis(10));
    timer.stop();
    
    assert_eq!(histogram.count(), 1);
    assert!(histogram.min() >= 10000); // At least 10ms in microseconds
}

#[test]
fn test_metrics_registry() {
    let registry = MetricsRegistry::new();
    
    // Get or create counters
    let counter1 = registry.counter("test-counter-1");
    let counter2 = registry.counter("test-counter-2");
    
    counter1.increment(10);
    counter2.increment(20);
    
    // Get the same counter again
    let counter1_again = registry.counter("test-counter-1");
    assert_eq!(counter1_again.value(), 10);
    
    // Create a histogram
    let histogram = registry.exponential_histogram("test-histogram", 1.0, 2.0, 3);
    histogram.record(5.0);
    
    // Create a timer
    let timer = registry.timer("test-timer");
    timer.stop();
    
    // Format metrics
    let metrics_str = registry.format();
    assert!(metrics_str.contains("test-counter-1: 10"));
    assert!(metrics_str.contains("test-counter-2: 20"));
    assert!(metrics_str.contains("test-histogram: count=1"));
    assert!(metrics_str.contains("test-timer: count=1"));
}

#[test]
fn test_metrics_collector() {
    let collector = MetricsCollector::new();
    
    // Record various events
    collector.record_connection("accepted");
    collector.record_connection("closed");
    
    collector.record_request("GET", 200);
    collector.record_request("POST", 404);
    
    collector.record_bytes_received(1024);
    collector.record_bytes_sent(2048);
    
    // Time a request
    {
        let _timer = collector.time_request("GET");
        thread::sleep(Duration::from_millis(1));
    }
    
    // Format metrics
    let metrics_str = collector.format();
    assert!(metrics_str.contains("connections.accepted: 1"));
    assert!(metrics_str.contains("connections.closed: 1"));
    assert!(metrics_str.contains("requests.GET.200: 1"));
    assert!(metrics_str.contains("requests.POST.404: 1"));
    assert!(metrics_str.contains("bytes_received: 1024"));
    assert!(metrics_str.contains("bytes_sent: 2048"));
    assert!(metrics_str.contains("request_time.GET: count=1"));
}

#[test]
fn test_registry_concurrent_access() {
    let registry = Arc::new(MetricsRegistry::new());
    let num_threads = 10;
    
    let mut handles = Vec::with_capacity(num_threads);
    
    for i in 0..num_threads {
        let registry_clone = registry.clone();
        let handle = thread::spawn(move || {
            let counter_name = format!("counter-{}", i);
            let counter = registry_clone.counter(&counter_name);
            counter.increment(i);
            
            let histogram_name = format!("histogram-{}", i);
            let histogram = registry_clone.exponential_histogram(&histogram_name, 1.0, 2.0, 3);
            histogram.record(i as f64);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all counters and histograms were created
    for i in 0..num_threads {
        let counter_name = format!("counter-{}", i);
        let counter = registry.counter(&counter_name);
        assert_eq!(counter.value(), i);
        
        let histogram_name = format!("histogram-{}", i);
        let histogram = registry.exponential_histogram(&histogram_name, 1.0, 2.0, 3);
        assert_eq!(histogram.count(), 1);
    }
}