#!/bin/bash

# IPPAN Deployment Script
# This script handles deployment of IPPAN nodes to various environments

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOYMENT_DIR="$PROJECT_ROOT/deployments"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
IPPAN Deployment Script

Usage: $0 [OPTIONS] COMMAND [ARGS...]

Commands:
    build           Build Docker images
    deploy          Deploy to specified environment
    rollback        Rollback deployment
    status          Check deployment status
    logs            View deployment logs
    test            Run deployment tests
    cleanup         Clean up deployment resources

Options:
    -e, --env       Environment (staging, production)
    -t, --tag       Docker image tag
    -n, --namespace Kubernetes namespace
    -h, --help      Show this help message

Examples:
    $0 build
    $0 deploy -e staging -t v1.0.0
    $0 deploy -e production -t latest
    $0 rollback -e staging
    $0 status -e production
    $0 logs -e staging
    $0 test -e staging
    $0 cleanup -e staging

EOF
}

# Default values
ENVIRONMENT=""
IMAGE_TAG="latest"
NAMESPACE="ippan"
COMMAND=""

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -e|--env)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -t|--tag)
                IMAGE_TAG="$2"
                shift 2
                ;;
            -n|--namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            build|deploy|rollback|status|logs|test|cleanup)
                COMMAND="$1"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Validate environment
validate_environment() {
    if [[ -z "$ENVIRONMENT" ]]; then
        log_error "Environment must be specified"
        exit 1
    fi
    
    case "$ENVIRONMENT" in
        staging|production)
            ;;
        *)
            log_error "Invalid environment: $ENVIRONMENT. Must be 'staging' or 'production'"
            exit 1
            ;;
    esac
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    # Check if kubectl is installed
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl is not installed"
        exit 1
    fi
    
    # Check if helm is installed
    if ! command -v helm &> /dev/null; then
        log_warning "Helm is not installed. Some features may not work."
    fi
    
    # Check Kubernetes connection
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Build Docker images
build_images() {
    log_info "Building Docker images..."
    
    cd "$PROJECT_ROOT"
    
    # Build production image
    log_info "Building production image with tag: $IMAGE_TAG"
    docker build -f Dockerfile.production -t "ippan:$IMAGE_TAG" .
    
    # Build development image
    log_info "Building development image"
    docker build -f Dockerfile -t "ippan:dev" .
    
    # Tag images for registry
    if [[ -n "${DOCKER_REGISTRY:-}" ]]; then
        log_info "Tagging images for registry: $DOCKER_REGISTRY"
        docker tag "ippan:$IMAGE_TAG" "$DOCKER_REGISTRY/ippan:$IMAGE_TAG"
        docker tag "ippan:dev" "$DOCKER_REGISTRY/ippan:dev"
    fi
    
    log_success "Docker images built successfully"
}

# Deploy to environment
deploy() {
    validate_environment
    check_prerequisites
    
    log_info "Deploying to $ENVIRONMENT environment..."
    
    # Create namespace if it doesn't exist
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
    
    # Apply Kubernetes manifests
    log_info "Applying Kubernetes manifests..."
    kubectl apply -f "$DEPLOYMENT_DIR/kubernetes/ippan-deployment.yaml" -n "$NAMESPACE"
    
    # Update image tag in deployment
    if [[ "$IMAGE_TAG" != "latest" ]]; then
        log_info "Updating image tag to: $IMAGE_TAG"
        kubectl set image deployment/ippan-node ippan-node="ippan:$IMAGE_TAG" -n "$NAMESPACE"
    fi
    
    # Apply monitoring if enabled
    if [[ "${ENABLE_MONITORING:-true}" == "true" ]]; then
        log_info "Applying monitoring manifests..."
        kubectl apply -f "$DEPLOYMENT_DIR/kubernetes/ippan-monitoring.yaml"
    fi
    
    # Wait for deployment to be ready
    log_info "Waiting for deployment to be ready..."
    kubectl rollout status deployment/ippan-node -n "$NAMESPACE" --timeout=300s
    
    # Run health checks
    log_info "Running health checks..."
    run_health_checks
    
    log_success "Deployment to $ENVIRONMENT completed successfully"
}

# Rollback deployment
rollback() {
    validate_environment
    check_prerequisites
    
    log_info "Rolling back deployment in $ENVIRONMENT environment..."
    
    # Rollback deployment
    kubectl rollout undo deployment/ippan-node -n "$NAMESPACE"
    
    # Wait for rollback to complete
    log_info "Waiting for rollback to complete..."
    kubectl rollout status deployment/ippan-node -n "$NAMESPACE" --timeout=300s
    
    # Run health checks
    log_info "Running health checks..."
    run_health_checks
    
    log_success "Rollback completed successfully"
}

# Check deployment status
status() {
    validate_environment
    check_prerequisites
    
    log_info "Checking deployment status in $ENVIRONMENT environment..."
    
    # Get deployment status
    kubectl get deployments -n "$NAMESPACE"
    echo
    
    # Get pod status
    kubectl get pods -n "$NAMESPACE"
    echo
    
    # Get service status
    kubectl get services -n "$NAMESPACE"
    echo
    
    # Get ingress status
    kubectl get ingress -n "$NAMESPACE" 2>/dev/null || true
    echo
    
    # Get replica set status
    kubectl get replicasets -n "$NAMESPACE"
    echo
    
    # Show deployment history
    log_info "Deployment history:"
    kubectl rollout history deployment/ippan-node -n "$NAMESPACE"
}

# View deployment logs
logs() {
    validate_environment
    check_prerequisites
    
    log_info "Viewing deployment logs in $ENVIRONMENT environment..."
    
    # Get pod names
    PODS=$(kubectl get pods -n "$NAMESPACE" -l app=ippan-node -o jsonpath='{.items[*].metadata.name}')
    
    if [[ -z "$PODS" ]]; then
        log_error "No pods found in namespace $NAMESPACE"
        exit 1
    fi
    
    # Show logs for each pod
    for pod in $PODS; do
        log_info "Logs for pod: $pod"
        kubectl logs "$pod" -n "$NAMESPACE" --tail=100
        echo
    done
}

# Run deployment tests
test() {
    validate_environment
    check_prerequisites
    
    log_info "Running deployment tests in $ENVIRONMENT environment..."
    
    # Get service endpoint
    SERVICE_IP=$(kubectl get service ippan-node-service -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}')
    SERVICE_PORT=$(kubectl get service ippan-node-service -n "$NAMESPACE" -o jsonpath='{.spec.ports[0].port}')
    
    if [[ -z "$SERVICE_IP" ]]; then
        log_error "Could not get service IP"
        exit 1
    fi
    
    # Test API endpoint
    log_info "Testing API endpoint: http://$SERVICE_IP:$SERVICE_PORT/api/v1/status"
    
    # Port forward for testing
    kubectl port-forward service/ippan-node-service 8080:$SERVICE_PORT -n "$NAMESPACE" &
    PORT_FORWARD_PID=$!
    
    # Wait for port forward to be ready
    sleep 5
    
    # Run tests
    if curl -f "http://localhost:8080/api/v1/status" > /dev/null 2>&1; then
        log_success "API endpoint test passed"
    else
        log_error "API endpoint test failed"
        kill $PORT_FORWARD_PID 2>/dev/null || true
        exit 1
    fi
    
    # Clean up port forward
    kill $PORT_FORWARD_PID 2>/dev/null || true
    
    log_success "Deployment tests passed"
}

# Run health checks
run_health_checks() {
    log_info "Running health checks..."
    
    # Check if pods are running
    RUNNING_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=ippan-node --field-selector=status.phase=Running --no-headers | wc -l)
    TOTAL_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=ippan-node --no-headers | wc -l)
    
    if [[ "$RUNNING_PODS" -eq "$TOTAL_PODS" && "$TOTAL_PODS" -gt 0 ]]; then
        log_success "All pods are running ($RUNNING_PODS/$TOTAL_PODS)"
    else
        log_error "Not all pods are running ($RUNNING_PODS/$TOTAL_PODS)"
        exit 1
    fi
    
    # Check if services are ready
    kubectl get services -n "$NAMESPACE" -l app=ippan-node
    
    log_success "Health checks passed"
}

# Clean up deployment resources
cleanup() {
    validate_environment
    check_prerequisites
    
    log_warning "Cleaning up deployment resources in $ENVIRONMENT environment..."
    
    # Delete deployments
    kubectl delete deployment ippan-node -n "$NAMESPACE" --ignore-not-found=true
    
    # Delete services
    kubectl delete service ippan-node-service -n "$NAMESPACE" --ignore-not-found=true
    kubectl delete service ippan-node-headless -n "$NAMESPACE" --ignore-not-found=true
    
    # Delete configmaps
    kubectl delete configmap ippan-config -n "$NAMESPACE" --ignore-not-found=true
    
    # Delete secrets
    kubectl delete secret ippan-keys -n "$NAMESPACE" --ignore-not-found=true
    
    # Delete PVCs
    kubectl delete pvc ippan-data -n "$NAMESPACE" --ignore-not-found=true
    
    # Delete namespace if empty
    if [[ "${DELETE_NAMESPACE:-false}" == "true" ]]; then
        kubectl delete namespace "$NAMESPACE" --ignore-not-found=true
    fi
    
    log_success "Cleanup completed"
}

# Main function
main() {
    parse_args "$@"
    
    if [[ -z "$COMMAND" ]]; then
        log_error "No command specified"
        show_help
        exit 1
    fi
    
    case "$COMMAND" in
        build)
            build_images
            ;;
        deploy)
            deploy
            ;;
        rollback)
            rollback
            ;;
        status)
            status
            ;;
        logs)
            logs
            ;;
        test)
            test
            ;;
        cleanup)
            cleanup
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
