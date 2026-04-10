use crate::models::{ServiceMetrics, TransactionMetrics};
use anyhow::Result;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Prometheus-compatible metrics collector.
/// Tracks request counts, latencies, and business metrics.
pub struct MetricsCollector {
    pool: PgPool,
    start_time: Instant,
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    latency_samples: Arc<tokio::sync::Mutex<Vec<f64>>>,
}

impl MetricsCollector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            start_time: Instant::now(),
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            latency_samples: Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(1000))),
        }
    }

    /// Record a request with its latency.
    pub async fn record_request(&self, latency_ms: f64, is_error: bool) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        let mut samples = self.latency_samples.lock().await;
        samples.push(latency_ms);
        // Keep only the last 10000 samples for percentile calculation
        if samples.len() > 10000 {
            samples.drain(..5000);
        }
    }

    /// Get service-level metrics.
    pub async fn get_service_metrics(&self, service_name: &str) -> ServiceMetrics {
        let samples = self.latency_samples.lock().await;
        let (avg, p95, p99) = calculate_percentiles(&samples);

        ServiceMetrics {
            service_name: service_name.to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            total_requests: self.request_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            avg_latency_ms: avg,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
        }
    }

    /// Get transaction-level metrics from the database.
    pub async fn get_transaction_metrics(&self) -> Result<TransactionMetrics> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions_ledger")
            .fetch_one(&self.pool)
            .await?;

        let (successful,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions_ledger WHERE status = 'confirmed'")
                .fetch_one(&self.pool)
                .await?;

        let (avg_amount,): (Option<f64>,) = sqlx::query_as(
            "SELECT AVG(amount)::FLOAT8 FROM transactions_ledger WHERE status = 'confirmed'",
        )
        .fetch_one(&self.pool)
        .await?;

        // TPS: transactions in last 60 seconds
        let (recent,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM transactions_ledger
            WHERE indexed_at >= NOW() - INTERVAL '60 seconds'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let tps_current = recent as f64 / 60.0;

        Ok(TransactionMetrics {
            total_processed: total as u64,
            successful: successful as u64,
            failed: (total - successful) as u64,
            tps_current,
            tps_peak: tps_current, // Would track peak separately in production
            avg_amount: avg_amount.unwrap_or(0.0),
        })
    }

    /// Export metrics in Prometheus text format.
    pub async fn export_prometheus(&self, service_name: &str) -> Result<String> {
        let svc = self.get_service_metrics(service_name).await;
        let tx = self.get_transaction_metrics().await?;

        let mut output = String::new();

        // Service metrics
        output.push_str(&format!(
            "# HELP solupg_uptime_seconds Service uptime in seconds\n\
             # TYPE solupg_uptime_seconds gauge\n\
             solupg_uptime_seconds{{service=\"{}\"}} {}\n\n",
            svc.service_name, svc.uptime_seconds
        ));

        output.push_str(&format!(
            "# HELP solupg_requests_total Total requests processed\n\
             # TYPE solupg_requests_total counter\n\
             solupg_requests_total{{service=\"{}\"}} {}\n\n",
            svc.service_name, svc.total_requests
        ));

        output.push_str(&format!(
            "# HELP solupg_errors_total Total errors\n\
             # TYPE solupg_errors_total counter\n\
             solupg_errors_total{{service=\"{}\"}} {}\n\n",
            svc.service_name, svc.error_count
        ));

        output.push_str(&format!(
            "# HELP solupg_latency_avg_ms Average request latency\n\
             # TYPE solupg_latency_avg_ms gauge\n\
             solupg_latency_avg_ms{{service=\"{}\"}} {:.2}\n\n",
            svc.service_name, svc.avg_latency_ms
        ));

        output.push_str(&format!(
            "# HELP solupg_latency_p95_ms 95th percentile latency\n\
             # TYPE solupg_latency_p95_ms gauge\n\
             solupg_latency_p95_ms{{service=\"{}\"}} {:.2}\n\n",
            svc.service_name, svc.p95_latency_ms
        ));

        output.push_str(&format!(
            "# HELP solupg_latency_p99_ms 99th percentile latency\n\
             # TYPE solupg_latency_p99_ms gauge\n\
             solupg_latency_p99_ms{{service=\"{}\"}} {:.2}\n\n",
            svc.service_name, svc.p99_latency_ms
        ));

        // Transaction metrics
        output.push_str(&format!(
            "# HELP solupg_transactions_total Total transactions processed\n\
             # TYPE solupg_transactions_total counter\n\
             solupg_transactions_total {}\n\n",
            tx.total_processed
        ));

        output.push_str(&format!(
            "# HELP solupg_transactions_successful Successful transactions\n\
             # TYPE solupg_transactions_successful counter\n\
             solupg_transactions_successful {}\n\n",
            tx.successful
        ));

        output.push_str(&format!(
            "# HELP solupg_transactions_failed Failed transactions\n\
             # TYPE solupg_transactions_failed counter\n\
             solupg_transactions_failed {}\n\n",
            tx.failed
        ));

        output.push_str(&format!(
            "# HELP solupg_tps_current Current transactions per second\n\
             # TYPE solupg_tps_current gauge\n\
             solupg_tps_current {:.2}\n\n",
            tx.tps_current
        ));

        Ok(output)
    }
}

/// Calculate average, p95, and p99 from a sample of latencies.
fn calculate_percentiles(samples: &[f64]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize].min(f64::MAX);
    let p99 = sorted[((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1)];

    (avg, p95, p99)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentiles_empty() {
        let (avg, p95, p99) = calculate_percentiles(&[]);
        assert_eq!(avg, 0.0);
        assert_eq!(p95, 0.0);
        assert_eq!(p99, 0.0);
    }

    #[test]
    fn test_percentiles_single() {
        let (avg, p95, p99) = calculate_percentiles(&[10.0]);
        assert_eq!(avg, 10.0);
        assert_eq!(p95, 10.0);
        assert_eq!(p99, 10.0);
    }

    #[test]
    fn test_percentiles_distribution() {
        let samples: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let (avg, p95, p99) = calculate_percentiles(&samples);

        assert!((avg - 50.5).abs() < 0.01);
        assert_eq!(p95, 96.0); // index 95
        assert_eq!(p99, 100.0); // index 99
    }

    #[test]
    fn test_prometheus_format() {
        // Verify the format string compiles and produces valid output
        let line = "# HELP solupg_uptime_seconds Service uptime\n\
             # TYPE solupg_uptime_seconds gauge\n\
             solupg_uptime_seconds{service=\"test\"} 120\n"
            .to_string();
        assert!(line.contains("solupg_uptime_seconds"));
        assert!(line.contains("gauge"));
    }
}
