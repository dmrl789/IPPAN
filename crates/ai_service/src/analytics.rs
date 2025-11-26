//! Analytics and insights module

use crate::errors::AIServiceError;
use crate::fixed_math::{correlation as fixed_correlation, mean, sqrt_fixed, variance};
use crate::types::{AnalyticsConfig, AnalyticsInsight, DataPoint, InsightType, SeverityLevel};
use chrono::{Duration, Utc};
use ippan_ai_core::Fixed;
use std::collections::HashMap;
use uuid::Uuid;

/// Analytics service
#[derive(Clone)]
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
    pub fn add_data_point(
        &mut self,
        metric: String,
        value: Fixed,
        unit: String,
        tags: HashMap<String, String>,
    ) {
        let data_point = DataPoint {
            metric: metric.clone(),
            value,
            unit,
            timestamp: Utc::now(),
            tags,
        };

        self.data_store.entry(metric).or_default().push(data_point);
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
        let values: Vec<Fixed> = recent_points.iter().map(|p| p.value).collect();

        let n = Fixed::from_int(values.len() as i64);
        let sum_x = (0..values.len()).fold(Fixed::ZERO, |acc, i| acc + Fixed::from_int(i as i64));
        let sum_y = values.iter().copied().fold(Fixed::ZERO, |acc, v| acc + v);
        let sum_xy = values.iter().enumerate().fold(Fixed::ZERO, |acc, (i, y)| {
            acc + (Fixed::from_int(i as i64) * *y)
        });
        let sum_x2 = (0..values.len()).fold(Fixed::ZERO, |acc, i| {
            let idx = Fixed::from_int(i as i64);
            acc + (idx * idx)
        });

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.is_zero() {
            return None;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let trend_strength = slope.abs();

        if trend_strength > Fixed::from_ratio(1, 10) {
            let trend_direction = if slope > Fixed::ZERO {
                "increasing"
            } else {
                "decreasing"
            };
            let severity = if trend_strength > Fixed::from_ratio(1, 2) {
                SeverityLevel::High
            } else {
                SeverityLevel::Medium
            };

            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("{metric} trend detected"),
                description: format!(
                    "{metric} is {trend_direction} with strength {trend_strength}"
                ),
                confidence: trend_strength.clamp(Fixed::ZERO, Fixed::ONE),
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
    fn analyze_anomalies(
        &self,
        metric: &str,
        data_points: &[DataPoint],
    ) -> Option<AnalyticsInsight> {
        if data_points.len() < 5 {
            return None;
        }

        let values: Vec<Fixed> = data_points.iter().map(|p| p.value).collect();
        let mean_value = mean(&values);
        let variance_value = variance(&values, mean_value);
        let std_dev = sqrt_fixed(variance_value);

        // Find outliers (values more than 2 standard deviations from mean)
        let outliers: Vec<&DataPoint> = data_points
            .iter()
            .filter(|&p| (p.value - mean_value).abs() > std_dev.mul_int(2))
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
                title: format!("Anomalies detected in {metric}"),
                description: format!(
                    "Found {} anomalous values in {} data points. Mean: {}, StdDev: {}",
                    outliers.len(),
                    data_points.len(),
                    mean_value,
                    std_dev
                ),
                confidence: Fixed::from_ratio(8, 10),
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
    fn analyze_performance(
        &self,
        metric: &str,
        data_points: &[DataPoint],
    ) -> Option<AnalyticsInsight> {
        if !metric.contains("latency")
            && !metric.contains("throughput")
            && !metric.contains("error")
        {
            return None;
        }

        let recent_points = &data_points[data_points.len().saturating_sub(20)..];
        let values: Vec<Fixed> = recent_points.iter().map(|p| p.value).collect();
        let avg_value = mean(&values);

        let (threshold, severity) = if metric.contains("latency") {
            (Fixed::from_int(1000), SeverityLevel::High) // 1 second
        } else if metric.contains("error") {
            (Fixed::from_ratio(5, 100), SeverityLevel::High) // 5% error rate
        } else if metric.contains("throughput") {
            (Fixed::from_int(100), SeverityLevel::Medium) // 100 TPS
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
                title: format!("Performance issue detected in {metric}"),
                description: format!(
                    "{} average value {} is {} threshold {}",
                    metric,
                    avg_value,
                    if metric.contains("throughput") {
                        "below"
                    } else {
                        "above"
                    },
                    threshold
                ),
                confidence: Fixed::from_ratio(9, 10),
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

        let values1: Vec<Fixed> = data1.iter().map(|p| p.value).collect();
        let values2: Vec<Fixed> = data2.iter().map(|p| p.value).collect();

        let correlation = fixed_correlation(&values1, &values2);

        if correlation.abs() > Fixed::from_ratio(7, 10) {
            let relationship = if correlation > Fixed::ZERO {
                "positive"
            } else {
                "negative"
            };

            Some(AnalyticsInsight {
                id: Uuid::new_v4().to_string(),
                insight_type: InsightType::Performance,
                title: format!("Strong correlation between {metric1} and {metric2}"),
                description: format!(
                    "Found {relationship} correlation ({correlation}) between {metric1} and {metric2}"
                ),
                confidence: correlation.abs().clamp(Fixed::ZERO, Fixed::ONE),
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
    #[allow(dead_code)]
    fn calculate_correlation(&self, x: &[Fixed], y: &[Fixed]) -> Fixed {
        fixed_correlation(x, y)
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
            .filter(|i| {
                std::mem::discriminant(&i.insight_type) == std::mem::discriminant(&insight_type)
            })
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

        service.add_data_point(
            "latency".to_string(),
            Fixed::from_int(100),
            "ms".to_string(),
            tags,
        );

        assert_eq!(service.data_store.len(), 1);
        assert_eq!(service.data_store["latency"].len(), 1);
        assert_eq!(service.data_store["latency"][0].value, Fixed::from_int(100));
    }

    #[test]
    fn test_correlation_calculation() {
        let service = AnalyticsService::new(Default::default());
        let x = vec![
            Fixed::from_int(1),
            Fixed::from_int(2),
            Fixed::from_int(3),
            Fixed::from_int(4),
            Fixed::from_int(5),
        ];
        let y = vec![
            Fixed::from_int(2),
            Fixed::from_int(4),
            Fixed::from_int(6),
            Fixed::from_int(8),
            Fixed::from_int(10),
        ];

        let correlation = service.calculate_correlation(&x, &y);
        assert!((correlation - Fixed::ONE).abs() < Fixed::from_ratio(1, 1000));
    }
}
