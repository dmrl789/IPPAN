//! Analytics and insights module

use crate::types::{
    AnalyticsInsight, InsightType, SeverityLevel, DataPoint, AnalyticsConfig
};
use crate::errors::AIServiceError;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use uuid::Uuid;

/// Analytics service
pub struct AnalyticsService {
    config: AnalyticsConfig,
    data_store: HashMap<String, Vec<DataPoint>>,
    insights: Vec<AnalyticsInsight>,
}

impl AnalyticsService {
    /// Create a new analytics service
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            data_store: HashMap::new(),
            insights: Vec::new(),
        }
    }

    /// Add data point
    pub fn add_data_point(&mut self, metric: String, value: f64, unit: String, tags: HashMap<String, String>) {
        let data_point = DataPoint {
            metric: metric.clone(),
            value,
            unit,
            timestamp: Utc::now(),
            tags,
        };

        self.data_store.entry(metric).or_insert_with(Vec::new).push(data_point);
    }

    /// Analyze data and generate insights
    pub async fn analyze(&mut self) -> Result<Vec<AnalyticsInsight>, AIServiceError> {
        let mut new_insights = Vec::new();

        // Analyze each metric
        for (metric, data_points) in &self.data_store {
            if data_points.len() < 2 {
                continue; // Need at least 2 points for analysis
            }

            // Detect trends
            if let Some(trend_insight) = self.analyze_trend(metric, data_points) {
                new_insights.push(trend_insight);
            }

            // Detect anomalies
            if let Some(anomaly_insight) = self.analyze_anomalies(metric, data_points) {
                new_insights.push(anomaly_insight);
            }

            // Performance analysis
            if let Some(perf_insight) = self.analyze_performance(metric, data_points) {
                new_insights.push(perf_insight);
            }
        }

        // Cross-metric analysis
        if let Some(cross_insight) = self.analyze_cross_metrics() {
            new_insights.push(cross_insight);
        }

        // Store insights
        self.insights.extend(new_insights.clone());

        // Clean up old data
        self.cleanup_old_data();

        Ok(new_insights)
    }

    /// Analyze trend for a metric
    fn analyze_trend(&self, metric: &str, data_points: &[DataPoint]) -> Option<AnalyticsInsight> {
        if data_points.len() < 10 {
            return None;
        }

        let recent_points = &data_points[data_points.len().saturating_sub(10)..];
        let values: Vec<f64> = recent_points.iter().map(|p| p.value).collect();

        // Simple linear regression to detect trend
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| (i as f64) * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let trend_strength = slope.abs();

        if trend_strength > 0.1 {
            let trend_direction = if slope > 0.0 { "increasing" } else { "decreasing" };
            let severity = if trend_strength > 0.5 { SeverityLevel::High } else { SeverityLevel::Medium };

            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("{} trend detected", metric),
                description: format!("{} is {} with strength {:.2}", metric, trend_direction, trend_strength),
                confidence: trend_strength.min(1.0),
                severity,
                data_points: recent_points.to_vec(),
                recommendations: vec![
                    format!("Monitor {} closely", metric),
                    "Consider investigating the cause of this trend".to_string(),
                ],
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Analyze anomalies in data
    fn analyze_anomalies(&self, metric: &str, data_points: &[DataPoint]) -> Option<AnalyticsInsight> {
        if data_points.len() < 5 {
            return None;
        }

        let values: Vec<f64> = data_points.iter().map(|p| p.value).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        // Find outliers (values more than 2 standard deviations from mean)
        let outliers: Vec<&DataPoint> = data_points
            .iter()
            .filter(|&p| (p.value - mean).abs() > 2.0 * std_dev)
            .collect();

        if !outliers.is_empty() {
            let severity = if outliers.len() > data_points.len() / 4 {
                SeverityLevel::High
            } else {
                SeverityLevel::Medium
            };

            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("Anomalies detected in {}", metric),
                description: format!(
                    "Found {} anomalous values in {} data points. Mean: {:.2}, StdDev: {:.2}",
                    outliers.len(),
                    data_points.len(),
                    mean,
                    std_dev
                ),
                confidence: 0.8,
                severity,
                data_points: outliers.into_iter().cloned().collect(),
                recommendations: vec![
                    "Investigate the cause of these anomalies".to_string(),
                    "Consider implementing alerting for similar patterns".to_string(),
                ],
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Analyze performance metrics
    fn analyze_performance(&self, metric: &str, data_points: &[DataPoint]) -> Option<AnalyticsInsight> {
        if !metric.contains("latency") && !metric.contains("throughput") && !metric.contains("error") {
            return None;
        }

        let recent_points = &data_points[data_points.len().saturating_sub(20)..];
        let values: Vec<f64> = recent_points.iter().map(|p| p.value).collect();
        let avg_value = values.iter().sum::<f64>() / values.len() as f64;

        let (threshold, severity) = if metric.contains("latency") {
            (1000.0, SeverityLevel::High) // 1 second
        } else if metric.contains("error") {
            (0.05, SeverityLevel::High) // 5% error rate
        } else if metric.contains("throughput") {
            (100.0, SeverityLevel::Medium) // 100 TPS
        } else {
            return None;
        };

        let is_problematic = if metric.contains("throughput") {
            avg_value < threshold
        } else {
            avg_value > threshold
        };

        if is_problematic {
            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("Performance issue detected in {}", metric),
                description: format!(
                    "{} average value {:.2} is {} threshold {:.2}",
                    metric,
                    avg_value,
                    if metric.contains("throughput") { "below" } else { "above" },
                    threshold
                ),
                confidence: 0.9,
                severity,
                data_points: recent_points.to_vec(),
                recommendations: vec![
                    format!("Optimize {} to improve performance", metric),
                    "Consider scaling resources if needed".to_string(),
                ],
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Analyze cross-metric relationships
    fn analyze_cross_metrics(&self) -> Option<AnalyticsInsight> {
        // Look for correlations between different metrics
        let metrics: Vec<(&String, &Vec<DataPoint>)> = self.data_store.iter().collect();
        
        if metrics.len() < 2 {
            return None;
        }

        // Simple correlation analysis between first two metrics
        let (metric1, data1) = &metrics[0];
        let (metric2, data2) = &metrics[1];

        if data1.len() != data2.len() || data1.len() < 5 {
            return None;
        }

        let values1: Vec<f64> = data1.iter().map(|p| p.value).collect();
        let values2: Vec<f64> = data2.iter().map(|p| p.value).collect();

        let correlation = self.calculate_correlation(&values1, &values2);

        if correlation.abs() > 0.7 {
            let relationship = if correlation > 0.0 { "positive" } else { "negative" };
            
            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("Strong correlation between {} and {}", metric1, metric2),
                description: format!(
                    "Found {} correlation ({:.2}) between {} and {}",
                    relationship,
                    correlation,
                    metric1,
                    metric2
                ),
                confidence: correlation.abs(),
                severity: SeverityLevel::Medium,
                data_points: data1.iter().chain(data2.iter()).cloned().collect(),
                recommendations: vec![
                    format!("Monitor both {} and {} together", metric1, metric2),
                    "Consider optimizing the underlying cause".to_string(),
                ],
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Calculate correlation coefficient
    fn calculate_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
        let sum_x2: f64 = x.iter().map(|a| a * a).sum();
        let sum_y2: f64 = y.iter().map(|a| a * a).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Clean up old data based on retention policy
    fn cleanup_old_data(&mut self) {
        let cutoff_time = Utc::now() - Duration::days(self.config.retention_days as i64);
        
        for data_points in self.data_store.values_mut() {
            data_points.retain(|p| p.timestamp > cutoff_time);
        }

        // Clean up old insights
        self.insights.retain(|i| i.timestamp > cutoff_time);
    }

    /// Get all insights
    pub fn get_insights(&self) -> &[AnalyticsInsight] {
        &self.insights
    }

    /// Get insights by type
    pub fn get_insights_by_type(&self, insight_type: InsightType) -> Vec<&AnalyticsInsight> {
        self.insights
            .iter()
            .filter(|i| std::mem::discriminant(&i.insight_type) == std::mem::discriminant(&insight_type))
            .collect()
    }

    /// Get recent insights
    pub fn get_recent_insights(&self, hours: i64) -> Vec<&AnalyticsInsight> {
        let cutoff_time = Utc::now() - Duration::hours(hours);
        self.insights
            .iter()
            .filter(|i| i.timestamp > cutoff_time)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_analytics_service_creation() {
        let config = AnalyticsConfig {
            enable_realtime: true,
            retention_days: 30,
            analysis_interval: 60,
            enable_predictive: true,
        };

        let service = AnalyticsService::new(config);
        assert_eq!(service.data_store.len(), 0);
        assert_eq!(service.insights.len(), 0);
    }

    #[test]
    fn test_add_data_point() {
        let mut service = AnalyticsService::new(Default::default());
        let mut tags = HashMap::new();
        tags.insert("node".to_string(), "node1".to_string());

        service.add_data_point("latency".to_string(), 100.0, "ms".to_string(), tags);
        
        assert_eq!(service.data_store.len(), 1);
        assert_eq!(service.data_store["latency"].len(), 1);
        assert_eq!(service.data_store["latency"][0].value, 100.0);
    }

    #[test]
    fn test_correlation_calculation() {
        let service = AnalyticsService::new(Default::default());
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        
        let correlation = service.calculate_correlation(&x, &y);
        assert!((correlation - 1.0).abs() < 0.001); // Should be perfect positive correlation
    }
}