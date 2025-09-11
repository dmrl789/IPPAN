@echo off
REM IPPAN Production Deployment Script for Windows
REM This script deploys IPPAN to production environment

setlocal enabledelayedexpansion

REM Configuration
set PROJECT_NAME=ippan
set ENVIRONMENT=production
set DOCKER_COMPOSE_FILE=deployments\production\docker-compose.production.yml
set ENV_FILE=.env.production

echo 🔐 Starting IPPAN production deployment...

REM Check prerequisites
echo 📋 Checking prerequisites...

docker --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Docker is not installed. Please install Docker Desktop first.
    pause
    exit /b 1
)

docker-compose --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Docker Compose is not installed. Please install Docker Compose first.
    pause
    exit /b 1
)

openssl version >nul 2>&1
if errorlevel 1 (
    echo ❌ OpenSSL is not installed. Please install OpenSSL first.
    pause
    exit /b 1
)

echo ✅ All prerequisites are met.

REM Generate SSL certificates
echo 🔐 Generating SSL certificates...
if exist "scripts\generate-ssl-certs.bat" (
    call scripts\generate-ssl-certs.bat
) else (
    echo ❌ SSL certificate generation script not found.
    pause
    exit /b 1
)

REM Create environment file
echo 📝 Creating environment file...
if not exist "%ENV_FILE%" (
    echo # IPPAN Production Environment Configuration > "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Database >> "%ENV_FILE%"
    echo POSTGRES_DB=ippan_production >> "%ENV_FILE%"
    echo POSTGRES_USER=ippan >> "%ENV_FILE%"
    echo POSTGRES_PASSWORD=changeme123 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Redis >> "%ENV_FILE%"
    echo REDIS_PASSWORD=changeme123 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # JWT >> "%ENV_FILE%"
    echo JWT_SECRET=changeme123 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Grafana >> "%ENV_FILE%"
    echo GRAFANA_ADMIN_PASSWORD=admin123 >> "%ENV_FILE%"
    echo GRAFANA_SECRET_KEY=changeme123 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Backup >> "%ENV_FILE%"
    echo BACKUP_ENCRYPTION_KEY=changeme123 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # SMTP (for alerts) >> "%ENV_FILE%"
    echo SMTP_PASSWORD= >> "%ENV_FILE%"
    echo EMAIL_SMTP_HOST=smtp.gmail.com >> "%ENV_FILE%"
    echo EMAIL_SMTP_PORT=587 >> "%ENV_FILE%"
    echo EMAIL_USERNAME=alerts@ippan.network >> "%ENV_FILE%"
    echo EMAIL_PASSWORD= >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Slack (for alerts) >> "%ENV_FILE%"
    echo SLACK_WEBHOOK_URL= >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Network >> "%ENV_FILE%"
    echo IPPAN_NETWORK_PORT=8080 >> "%ENV_FILE%"
    echo IPPAN_API_PORT=3000 >> "%ENV_FILE%"
    echo IPPAN_P2P_PORT=8080 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Security >> "%ENV_FILE%"
    echo IPPAN_ENABLE_TLS=true >> "%ENV_FILE%"
    echo IPPAN_ENABLE_MUTUAL_AUTH=true >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Performance >> "%ENV_FILE%"
    echo IPPAN_MAX_CONNECTIONS=1000 >> "%ENV_FILE%"
    echo IPPAN_THREAD_POOL_SIZE=8 >> "%ENV_FILE%"
    echo IPPAN_CACHE_SIZE=2147483648 >> "%ENV_FILE%"
    echo IPPAN_MEMORY_POOL_SIZE=1073741824 >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Monitoring >> "%ENV_FILE%"
    echo PROMETHEUS_RETENTION=30d >> "%ENV_FILE%"
    echo GRAFANA_ADMIN_USER=admin >> "%ENV_FILE%"
    echo ALERTMANAGER_CONFIG_FILE=/etc/alertmanager/alertmanager.yml >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Logging >> "%ENV_FILE%"
    echo RUST_LOG=info >> "%ENV_FILE%"
    echo LOG_LEVEL=info >> "%ENV_FILE%"
    echo LOG_FORMAT=json >> "%ENV_FILE%"
    echo. >> "%ENV_FILE%"
    echo # Production flags >> "%ENV_FILE%"
    echo NODE_ENV=production >> "%ENV_FILE%"
    echo IPPAN_ENVIRONMENT=production >> "%ENV_FILE%"
    echo IPPAN_ENABLE_METRICS=true >> "%ENV_FILE%"
    echo IPPAN_ENABLE_HEALTH_CHECKS=true >> "%ENV_FILE%"
    echo ✅ Environment file created: %ENV_FILE%
) else (
    echo ⚠️ Environment file already exists: %ENV_FILE%
)

REM Build Docker images
echo 🏗️ Building Docker images...
docker build -f Dockerfile.production -t ippan/ippan:latest .
if errorlevel 1 (
    echo ❌ Failed to build IPPAN Docker image.
    pause
    exit /b 1
)

REM Pull monitoring images
docker pull prom/prometheus:latest
docker pull grafana/grafana:latest
docker pull prom/alertmanager:latest
docker pull fluent/fluentd:v1.16-debian-1
docker pull nginx:alpine
docker pull alpine:latest

echo ✅ Docker images built successfully.

REM Deploy services
echo 🚀 Deploying IPPAN services...

REM Stop existing services
docker-compose -f "%DOCKER_COMPOSE_FILE%" --env-file "%ENV_FILE%" down

REM Start services
docker-compose -f "%DOCKER_COMPOSE_FILE%" --env-file "%ENV_FILE%" up -d
if errorlevel 1 (
    echo ❌ Failed to deploy IPPAN services.
    pause
    exit /b 1
)

echo ✅ IPPAN services deployed successfully.

REM Wait for services to be ready
echo ⏳ Waiting for services to be ready...

REM Wait for IPPAN node
echo ⏳ Waiting for IPPAN node...
timeout /t 60 /nobreak >nul
:wait_ippan
curl -f http://localhost:3000/api/v1/status >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_ippan
)

REM Wait for Prometheus
echo ⏳ Waiting for Prometheus...
:wait_prometheus
curl -f http://localhost:9090/-/healthy >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_prometheus
)

REM Wait for Grafana
echo ⏳ Waiting for Grafana...
:wait_grafana
curl -f http://localhost:3001/api/health >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_grafana
)

echo ✅ All services are ready.

REM Verify deployment
echo 🔍 Verifying deployment...

REM Check IPPAN node health
curl -f http://localhost:3000/api/v1/status >nul 2>&1
if errorlevel 1 (
    echo ❌ IPPAN node health check failed
    pause
    exit /b 1
) else (
    echo ✅ IPPAN node is healthy
)

REM Check Prometheus
curl -f http://localhost:9090/-/healthy >nul 2>&1
if errorlevel 1 (
    echo ❌ Prometheus health check failed
    pause
    exit /b 1
) else (
    echo ✅ Prometheus is healthy
)

REM Check Grafana
curl -f http://localhost:3001/api/health >nul 2>&1
if errorlevel 1 (
    echo ❌ Grafana health check failed
    pause
    exit /b 1
) else (
    echo ✅ Grafana is healthy
)

REM Check Nginx
curl -f http://localhost/health >nul 2>&1
if errorlevel 1 (
    echo ❌ Nginx health check failed
    pause
    exit /b 1
) else (
    echo ✅ Nginx is healthy
)

echo ✅ Deployment verification completed successfully.

REM Show deployment information
echo.
echo 🌐 IPPAN Production Deployment Information:
echo.
echo 🌐 IPPAN Node API: http://localhost:3000
echo 📊 Prometheus: http://localhost:9090
echo 📈 Grafana: http://localhost:3001
echo 🔔 AlertManager: http://localhost:9093
echo 🌍 Load Balancer: http://localhost (HTTP) / https://localhost (HTTPS)
echo.
echo 📋 Useful Commands:
echo   View logs: docker-compose -f %DOCKER_COMPOSE_FILE% logs -f
echo   Stop services: docker-compose -f %DOCKER_COMPOSE_FILE% down
echo   Restart services: docker-compose -f %DOCKER_COMPOSE_FILE% restart
echo   Scale services: docker-compose -f %DOCKER_COMPOSE_FILE% up -d --scale ippan-node=3
echo.
echo 🔐 Security Notes:
echo   - SSL certificates are generated in deployments\ssl\
echo   - Environment variables are in %ENV_FILE%
echo   - Keep private keys secure and never commit them to version control
echo   - Change default passwords in production
echo.
echo 📊 Monitoring:
echo   - Grafana admin password: Check %ENV_FILE%
echo   - Prometheus metrics: http://localhost:9090/metrics
echo   - Alert rules: deployments\monitoring\ippan-production-rules.yml
echo.

echo 🚀 IPPAN production deployment completed successfully!
pause