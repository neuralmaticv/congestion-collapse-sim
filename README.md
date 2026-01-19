# Congestion Collapse Simulator

## To-Do - Future enhancements

- [ ] Add detailed metrics tracking (p50/p95/p99 latencies, queue depth over time, worker utilization)
- [ ] Implement CSV/JSON output for metrics to enable external visualization
- [ ] Add priority queue support with request priorities
- [ ] Implement circuit breaker pattern with configurable thresholds
- [ ] Add exponential backoff for retries instead of fixed probability
- [ ] Implement load shedding strategies (drop oldest, random, etc.)
- [ ] Add multi-tier service simulation (client -> service A -> service B)
- [ ] **Nice to Have:** Create terminal-based visualization using ratatui or similar

## Overview

This simulator explores the behavior of synchronous request/reply systems under different loads and configuration parameters. It models realistic server scenarios where:

- Requests arrive at a variable rate
- A fixed number of workers process requests
- Requests wait in a queue when all workers are busy
- Requests can timeout if they wait too long
- Failed requests can be retried (with configurable probability)
- Temporary latency spikes can be simulated to stress-test the system

## Why this matters

In synchronous systems, when a large enough queue builds up, requests may timeout before being processed. Clients typically retry failed requests, creating more load on an already overwhelmed system. This leads to **congestion collapse** - where the server does wasted work processing requests that clients have already given up on.

This simulator helps you understand:
- The impact of queueing on system availability
- How different queue strategies (FIFO vs LIFO) affect failure rates
- When load shedding or cooperative clients are necessary
- The relationship between throughput and response time

> **Reference**: *Designing Data-Intensive Applications* by Martin Kleppmann
> "As throughput approaches the maximum that the hardware can handle, queueing delays increase sharply."

## Installation

```bash
git clone https://github.com/neuralmaticv/congestion-collapse-sim
cd congestion-collapse-sim
cargo build --release
```

## Usage

```bash
cargo run --release -- [OPTIONS]
```

### Core parameters

- `-r, --arrival-rate <RATE>` - Mean arrival rate of new requests per tick (**required**)
- `-w, --workers <NUM>` - Number of workers processing requests (default: 10)
- `-q, --queue-size <SIZE>` - Maximum size of the request queue (default: 50)
- `-t, --timeout <TICKS>` - Request timeout in ticks (default: 1000)
- `--mean-latency <TICKS>` - Mean processing latency per request (default: 50)
- `--ticks <NUM>` - Number of simulation ticks to run (default: 1,000,000)

### Behavior flags

- `--lifo` - Use LIFO queue instead of FIFO (helps during overload!)
- `--simulate-spike` - Simulate temporary 10x latency spike (first 0.1% of ticks)
- `--retry-probability <PROB>` - Probability failed requests retry (default: 0.5)

### Examples

**Baseline - Low load, no failures:**
```bash
cargo run --release -- -r 0.1
# Failure rate: ~0.00%
```

**Moderate load with spike - FIFO vs LIFO comparison:**
```bash
# FIFO (default) - higher failure rate
cargo run --release -- -r 0.1 --simulate-spike
# Failure rate: ~9.87%

# LIFO - lower failure rate (serves fresh requests first!)
cargo run --release -- -r 0.1 --simulate-spike --lifo
# Failure rate: ~0.79%
```

**Severe overload - congestion collapse:**
```bash
cargo run --release -- -r 0.5
# Failure rate: ~86.76%
```

## How this works

The simulator uses a **discrete-time model** where each "tick" represents one unit of time:

1. **Age queue**: All queued requests move one tick closer to timeout
2. **Generate requests**: New requests arrive based on normal distribution around arrival rate
3. **Assign work**: Idle workers pick up requests (FIFO or LIFO)
4. **Process requests**: Workers tick their current requests toward completion
5. **Handle timeouts**: Finished-but-timed-out requests are marked as failures
6. **Retry logic**: Failed requests may be retried based on probability

### Key insight: LIFO during overload

When the system is overloaded:
- **FIFO**: Old requests (already waited long) are served first → high timeout risk
- **LIFO**: Fresh requests are served first → lower timeout risk

LIFO sacrifices fairness for availability under stress!

## References and literature:
- Martin Kleppmann, *Designing Data-Intensive Applications*, Chapter 2
- Chapter 1 (pages 13–17): Describing Performance
- Queueing theory: M/M/c queue model

---
## License

[MIT License](LICENSE)
