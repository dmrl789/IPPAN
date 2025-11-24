# ============================================
# IPPAN Unified UI — Production Dockerfile
# Static export served with Nginx
# ============================================

# ---- Build stage ----
FROM node:20-alpine AS builder
ARG NEXT_PUBLIC_API_BASE_URL="http://localhost:8081/api"
ARG NEXT_PUBLIC_WS_URL="ws://localhost:8081/ws"
ARG NEXT_PUBLIC_GATEWAY_URL="http://localhost:8081/api"
ARG NEXT_PUBLIC_NETWORK_NAME="IPPAN Devnet"
ARG NEXT_PUBLIC_ENABLE_FULL_UI="1"
ENV NEXT_PUBLIC_API_BASE_URL=${NEXT_PUBLIC_API_BASE_URL}
ENV NEXT_PUBLIC_WS_URL=${NEXT_PUBLIC_WS_URL}
ENV NEXT_PUBLIC_GATEWAY_URL=${NEXT_PUBLIC_GATEWAY_URL}
ENV NEXT_PUBLIC_NETWORK_NAME=${NEXT_PUBLIC_NETWORK_NAME}
ENV NEXT_PUBLIC_ENABLE_FULL_UI=${NEXT_PUBLIC_ENABLE_FULL_UI}
WORKDIR /app

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies
RUN npm ci

# Copy source files
COPY . .

# Build static export
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm run build

# ---- Runtime stage ----
FROM nginx:alpine AS runner

# Copy static files from build
COPY --from=builder /app/out /usr/share/nginx/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
