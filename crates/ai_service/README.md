# IPPAN AI Service

High-level AI service layer providing intelligent capabilities for the IPPAN blockchain network.

## Features

- **LLM Integration**: Natural language processing and text generation
- **Smart Contract Analysis**: AI-powered contract analysis and optimization
- **Blockchain Analytics**: Real-time analytics and insights
- **Transaction Optimization**: AI-driven transaction optimization
- **Intelligent Monitoring**: Advanced monitoring and alerting

## Quick Start

### Prerequisites

- Rust 1.75+
- Docker and Docker Compose
- LLM API key (OpenAI, Anthropic, etc.)

### Development

```bash
# Clone the repository
git clone <repository-url>
cd ippan-ai-service

# Install dependencies
cargo build

# Run tests
cargo test

# Run the service
cargo run --bin ippan-ai-service
```

### Production Deployment

```bash
# Build and deploy
./scripts/deploy.sh latest production

# Or using Docker Compose
docker-compose -f docker-compose.prod.yml up -d
```

## Configuration

The service supports multiple configuration methods:

### Environment Variables

The service reads the following environment variables at startup:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `IPPAN_ENV` | Environment (`development`, `staging`, `production`, `testing`) | `development` | Yes |
| `LLM_API_KEY` | API key used for outbound LLM requests (also accepted as `IPPAN_SECRET_LLM_API_KEY`) | - | Yes |
| `LLM_API_ENDPOINT` | Base URL for the LLM provider | `https://api.openai.com/v1` | No |
| `LLM_MODEL` | LLM model name | `gpt-4` | No |
| `LLM_MAX_TOKENS` | Maximum tokens per completion | `4000` | No |
| `LLM_TEMPERATURE` | Sampling temperature for completions | `0.7` | No |
| `LLM_TIMEOUT` | LLM request timeout (seconds) | `30` | No |
| `ENABLE_LLM` | Toggle LLM features | `true` | No |
| `ENABLE_ANALYTICS` | Toggle analytics engine | `true` | No |
| `ENABLE_SMART_CONTRACTS` | Toggle smart-contract analysis | `true` | No |
| `ENABLE_MONITORING` | Toggle monitoring pipeline | `true` | No |
| `MONITORING_INTERVAL` | Metrics emission interval (seconds) | `30` | No |
| `PROMETHEUS_ENDPOINT` | Remote write endpoint for Prometheus exporter (production) | `http://prometheus:9090/api/v1/write` | Yes (production) |
| `JSON_EXPORTER_ENDPOINT` | URL used by the JSON exporter | `http://localhost:8080/metrics` | No |
| `HEALTH_PORT` | Port for the `/health` and `/metrics` endpoints | `8080` | No |

Secrets can also be provided via prefixed variables (e.g. `IPPAN_SECRET_LLM_API_KEY`) or mounted files in `secrets/`. The configuration manager validates that an LLM API key is present before the service starts.

For local development you can copy `.env.example` to `.env` and adjust the values as needed.

### Configuration Files

Create configuration files in the `config/` directory:

- `config/production.toml` - Production settings
- `config/staging.toml` - Staging settings
- `config/development.toml` - Development settings

### Example Configuration

```toml
[service]
enable_llm = true
enable_analytics = true
enable_smart_contracts = true
enable_monitoring = true

[llm]
api_endpoint = "https://api.openai.com/v1"
model_name = "gpt-4"
max_tokens = 4000
temperature = 0.7
timeout_seconds = 30

[analytics]
enable_realtime = true
retention_days = 30
analysis_interval = 60
enable_predictive = true

[monitoring]
enable_anomaly_detection = true
monitoring_interval = 30
enable_auto_remediation = false

[monitoring.alert_thresholds]
memory_usage = 80.0
cpu_usage = 90.0
error_rate = 5.0
response_time = 1000.0
```

## API Endpoints

### Health Check

```bash
GET /health
```

Returns service health status and metrics.

### Metrics

```bash
GET /metrics
```

Returns Prometheus-formatted metrics.

## Usage Examples

### LLM Integration

```rust
use ippan_ai_service::{AIService, AIServiceConfig, LLMRequest};

let config = AIServiceConfig::default();
let mut service = AIService::new(config)?;
service.start().await?;

let request = LLMRequest {
    prompt: "Explain blockchain technology".to_string(),
    context: None,
    max_tokens: Some(500),
    temperature: Some(0.7),
    stream: false,
};

let response = service.generate_text(request).await?;
println!("Response: {}", response.text);
```

### Smart Contract Analysis

```rust
use ippan_ai_service::{SmartContractAnalysisRequest, ContractAnalysisType};

let request = SmartContractAnalysisRequest {
    code: contract_code.to_string(),
    language: "solidity".to_string(),
    analysis_type: ContractAnalysisType::Security,
    context: None,
};

let analysis = service.analyze_smart_contract(request).await?;
println!("Security Score: {}", analysis.security_score);
```

### Transaction Optimization

```rust
use ippan_ai_service::{TransactionOptimizationRequest, OptimizationGoal};

let request = TransactionOptimizationRequest {
    transaction: transaction_data,
    goals: vec![OptimizationGoal::MinimizeGas, OptimizationGoal::MinimizeCost],
    constraints: None,
};

let optimization = service.optimize_transaction(request).await?;
println!("Suggestions: {:?}", optimization.suggestions);
```

## Monitoring

The service includes comprehensive monitoring capabilities:

- **Health Checks**: Automatic health monitoring
- **Metrics Collection**: Prometheus-compatible metrics
- **Alerting**: Configurable alerts and thresholds
- **Logging**: Structured logging with multiple formats

### Grafana Dashboards

Pre-configured Grafana dashboards are available in `monitoring/grafana/dashboards/`.

### Prometheus Metrics

The service exposes metrics at `/metrics` endpoint:

- `ippan_ai_requests_total` - Total number of requests
- `ippan_ai_request_duration_seconds` - Request duration
- `ippan_ai_errors_total` - Total number of errors
- `ippan_ai_memory_usage_bytes` - Memory usage
- `ippan_ai_cpu_usage_percent` - CPU usage

## Security

- **Authentication**: Configurable authentication mechanisms
- **Encryption**: End-to-end encryption support
- **Secrets Management**: Secure secret handling
- **Access Control**: Role-based access control

## Performance

- **Async Processing**: Fully asynchronous architecture
- **Connection Pooling**: Efficient resource utilization
- **Caching**: Intelligent caching strategies
- **Load Balancing**: Built-in load balancing support

## Troubleshooting

### Common Issues

1. **Service won't start**
   - Check configuration files
   - Verify environment variables
   - Check logs for errors

2. **LLM requests failing**
   - Verify API key is correct
   - Check network connectivity
   - Verify API endpoint URL

3. **High memory usage**
   - Adjust memory limits
   - Check for memory leaks
   - Monitor garbage collection

### Logs

Logs are available in multiple formats:

- **JSON**: Structured logging for production
- **Pretty**: Human-readable format for development

Set `LOG_FORMAT=json` for production environments.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For support and questions:

- Create an issue on GitHub
- Join our Discord community
- Check the documentation

## Changelog

### v0.1.0
- Initial release
- LLM integration
- Smart contract analysis
- Basic monitoring
- Transaction optimization