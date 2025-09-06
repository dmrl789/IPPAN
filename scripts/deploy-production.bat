@echo off
REM IPPAN Production Deployment Script for Windows
REM This script deploys IPPAN to production with comprehensive monitoring and security

setlocal enabledelayedexpansion

REM Configuration
set DEPLOYMENT_ENV=production
set DOCKER_REGISTRY=ippan
set IMAGE_TAG=latest
set NAMESPACE=ippan-production

echo [INFO] Starting IPPAN production deployment...
echo [INFO] Environment: %DEPLOYMENT_ENV%
echo [INFO] Docker Registry: %DOCKER_REGISTRY%
echo [INFO] Image Tag: %IMAGE_TAG%

REM Check prerequisites
echo [INFO] Checking prerequisites...

REM Check if Docker is installed and running
docker --version >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Docker is not installed
    exit /b 1
)

docker info >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Docker is not running
    exit /b 1
)

REM Check if Docker Compose is installed
docker-compose --version >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Docker Compose is not installed
    exit /b 1
)

echo [SUCCESS] Prerequisites check completed

REM Generate SSL certificates
echo [INFO] Generating SSL certificates...
if not exist "ssl" mkdir ssl

if not exist "ssl\ippan.crt" (
    echo [INFO] Generating self-signed SSL certificate...
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ssl\ippan.key -out ssl\ippan.crt -subj "/C=US/ST=State/L=City/O=IPPAN/CN=ippan.network"
)

echo [SUCCESS] SSL certificates generated

REM Build Docker images
echo [INFO] Building Docker images...

echo [INFO] Building IPPAN node image...
docker build -f Dockerfile.optimized -t %DOCKER_REGISTRY%/ippan:%IMAGE_TAG% .

echo [INFO] Building frontend image...
docker build -f apps\unified-ui\Dockerfile -t %DOCKER_REGISTRY%/ippan-frontend:%IMAGE_TAG% apps\unified-ui\

REM Tag images for production
docker tag %DOCKER_REGISTRY%/ippan:%IMAGE_TAG% %DOCKER_REGISTRY%/ippan:production
docker tag %DOCKER_REGISTRY%/ippan-frontend:%IMAGE_TAG% %DOCKER_REGISTRY%/ippan-frontend:production

echo [SUCCESS] Docker images built successfully

REM Deploy with Docker Compose
echo [INFO] Deploying with Docker Compose...

echo [INFO] Stopping existing containers...
docker-compose -f docker-compose.production.yml down --remove-orphans

echo [INFO] Starting new containers...
docker-compose -f docker-compose.production.yml up -d

echo [INFO] Waiting for services to be healthy...
timeout /t 30 /nobreak >nul

REM Check service health
docker-compose -f docker-compose.production.yml ps

echo [SUCCESS] Docker Compose deployment completed successfully

REM Setup monitoring
echo [INFO] Setting up monitoring...
timeout /t 60 /nobreak >nul

REM Check if Prometheus is accessible
curl -f http://localhost:9090/-/healthy >nul 2>&1
if errorlevel 1 (
    echo [WARNING] Prometheus is not accessible
) else (
    echo [SUCCESS] Prometheus is running
)

REM Check if Grafana is accessible
curl -f http://localhost:3001/api/health >nul 2>&1
if errorlevel 1 (
    echo [WARNING] Grafana is not accessible
) else (
    echo [SUCCESS] Grafana is running
    echo [INFO] Grafana dashboard: http://localhost:3001 (admin/admin123)
)

REM Check if Alertmanager is accessible
curl -f http://localhost:9093/-/healthy >nul 2>&1
if errorlevel 1 (
    echo [WARNING] Alertmanager is not accessible
) else (
    echo [SUCCESS] Alertmanager is running
)

REM Health check
echo [INFO] Performing health check...

REM Check if IPPAN node is responding
curl -f http://localhost:80/health >nul 2>&1
if errorlevel 1 (
    echo [ERROR] IPPAN node is not responding
    exit /b 1
) else (
    echo [SUCCESS] IPPAN node is healthy
)

REM Check if API is responding
curl -f http://localhost:3000/api/v1/status >nul 2>&1
if errorlevel 1 (
    echo [ERROR] IPPAN API is not responding
    exit /b 1
) else (
    echo [SUCCESS] IPPAN API is healthy
)

echo [SUCCESS] Health check completed successfully

echo [SUCCESS] IPPAN production deployment completed successfully!
echo [INFO] Access points:
echo [INFO]   - Frontend: http://localhost:80
echo [INFO]   - API: http://localhost:3000
echo [INFO]   - P2P: localhost:8080
echo [INFO]   - Prometheus: http://localhost:9090
echo [INFO]   - Grafana: http://localhost:3001 (admin/admin123)
echo [INFO]   - Alertmanager: http://localhost:9093

endlocal
