use clap::Parser;
use std::collections::VecDeque;
use rand::{rng, Rng};
use rand_distr::Distribution;
use rand_distr::Normal;


#[derive(Debug, Parser)]
#[command(
    name = "congestion-collapse-sim",
    about = "A discrete-time queueing system simulator for modeling request processing, timeouts, and congestion collapse in synchronous request/reply systems"
)]
struct Config {
    // ========== Core simulation parameters ==========

    /// Number of simulation ticks to run
    #[arg(long = "ticks", default_value = "1000000")]
    simulation_ticks: u32,

    /// Number of workers to simulate
    #[arg(short = 'w', long = "workers", default_value = "10")]
    num_workers: usize,

    /// Maximum size of the request queue
    #[arg(short = 'q', long = "queue-size", default_value = "50")]
    queue_size: usize,

    // ========== Request parameters ==========

    /// Rate at which new requests arrive (must be > 0)
    #[arg(short = 'r', long = "arrival-rate")]
    request_arrival_rate: f64,

    /// Mean request processing latency in ticks (must be > 0)
    #[arg(long = "mean-latency", default_value = "50")]
    mean_request_latency: f64,

    /// Number of ticks before a request times out
    /// Should be larger than mean_latency for meaningful simulation
    #[arg(short = 't', long = "timeout", default_value = "1000")]
    request_timeout: u32,

    // ========== Behavior flags ==========

    /// Use LIFO queue instead of FIFO
    #[arg(long = "lifo")]
    lifo: bool,

    /// Simulate a temporary 10x latency spike for the first 0.1% of ticks
    #[arg(long = "simulate-spike")]
    simulate_spike: bool,

    /// Probability that a failed request will be retried (must be in [0, 1])
    #[arg(long = "retry-probability", default_value = "0.5")]
    retry_probability: f64,
}

/// Represents a client request being processed by the queueing system.
struct Request {
    remaining_ticks: u32,
    timeout_ticks: u32,
}

/// Represents a worker that processes requests from the queue.
struct Worker {
    _id: usize,
    current_request: Option<Request>,
}

impl Request {
    fn new(execution_time: u32, timeout: u32) -> Request {
        Request {
            remaining_ticks: execution_time,
            timeout_ticks: timeout,
        }
    }

    fn waiting_tick(&mut self) {
        if self.timeout_ticks > 0 {
            self.timeout_ticks -= 1;
        }
    }

    fn working_tick(&mut self) {
        if self.timeout_ticks > 0 {
            self.timeout_ticks -= 1;
        }

        if self.remaining_ticks > 0 {
            self.remaining_ticks -= 1;
        }
    }

    fn is_timed_out(&self) -> bool {
        self.timeout_ticks == 0
    }

    fn is_done(&self) -> bool {
        self.remaining_ticks == 0
    }
}

impl Worker {
    fn new(id: usize) -> Worker {
        Worker {
            _id: id,
            current_request: None,
        }
    }

    /// Processes one simulation tick.
    fn tick(&mut self, queue: &mut VecDeque<Request>, lifo: bool) -> Option<Request> {
        if let Some(current) = &mut self.current_request {
            current.working_tick();
            if current.is_done() {
                return self.current_request.take();
            }
        } else {
            let next = if lifo {
                queue.pop_back()
            } else {
                queue.pop_front()
            };
            self.current_request = next;
        }

        None
    }

    fn is_free(&self) -> bool {
        self.current_request.is_none()
    }

    fn take(&mut self, request: Request) {
        self.current_request = Some(request);
    }
}

fn maybe_retry(retry_probability: f64, incoming_requests: &mut f64) {
    if rng().random_bool(retry_probability) {
        *incoming_requests += 1.0;
    }
}

fn main() {
    const SPIKE_DURATION_FRACTION: u32 = 1000; // 0.1% of total ticks

    let config = Config::parse();

    if config.request_arrival_rate <= 0.0 {
        panic!("Error: request arrival rate must be greater than 0");
    }

    if config.mean_request_latency <= 0.0 {
        panic!("Error: mean request latency must be greater than 0");
    }

    if config.retry_probability < 0.0 || config.retry_probability > 1.0 {
        panic!("Error: retry probability must be between 0.0 and 1.0");
    }

    let mut queue: VecDeque<Request> = VecDeque::with_capacity(config.queue_size);
    let mut workers: Vec<Worker> = (0..config.num_workers).map(Worker::new).collect();

    // Note: Normal distribution should be replaced!
    let arrival_distribution = Normal::new(
        config.request_arrival_rate,
        config.request_arrival_rate / 4.0,
    ).unwrap();
    let latency_distribution = Normal::new(
        config.mean_request_latency,
        config.mean_request_latency / 4.0,
    ).unwrap();

    let mut total_requests: u32 = 0;
    let mut failed_requests: u32 = 0;
    let mut remaining_spike_ticks: u32;

    if config.simulate_spike {
        remaining_spike_ticks = config.simulation_ticks / SPIKE_DURATION_FRACTION;
    } else {
        remaining_spike_ticks = 0;
    }

    let mut incoming_requests: f64 = 0.0;
    for _ in 0..config.simulation_ticks {
        // Age all queued requests (one tick closer to timeout).
        queue.iter_mut().for_each(Request::waiting_tick);

        // Accumulate fractional requests to avoid losing precision.
        incoming_requests += arrival_distribution.sample(&mut rng());

        while incoming_requests > 0.0 {
            incoming_requests -= 1.0;
            total_requests += 1;

            // Clamp to zero since normal distribution can produce negative values.
            let mut execution_time = 0.0_f64.max(latency_distribution.sample(&mut rng()));

            // Apply 10x latency multiplier during spike period.
            if remaining_spike_ticks > 0 {
                remaining_spike_ticks -= 1;
                execution_time *= 10.0;
            }

            let request = Request::new(execution_time as u32, config.request_timeout);
            let idle_worker = workers.iter_mut().find(|w| w.is_free());

            if let Some(worker) = idle_worker {
                worker.take(request);
            } else if queue.len() < config.queue_size {
                queue.push_back(request);
            } else {
                // Request failed: queue full and all workers busy.
                failed_requests += 1;
                maybe_retry(config.retry_probability, &mut incoming_requests);
            }
        }

        // Process one tick for each worker.
        for worker in workers.iter_mut() {
            if let Some(finished_request) = worker.tick(&mut queue, config.lifo) {
                if finished_request.is_timed_out() {
                    // Worst case: request finished but timed out (client already gave up).
                    failed_requests += 1;
                    maybe_retry(config.retry_probability, &mut incoming_requests);
                }
            }
        }
    }

    let failure_rate = if total_requests > 0 {
        (failed_requests as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    println!("Simulation complete");
    println!("Total requests: {}", total_requests);
    println!("Failed requests: {}", failed_requests);
    println!("Failure rate: {:.2}%", failure_rate);
}
