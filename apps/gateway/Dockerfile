# IPPAN Gateway Dockerfile
# Multi-stage build for minimal image size and security

FROM node:20-alpine AS deps

ENV NODE_ENV=production
ENV PORT=8080

WORKDIR /app

# Copy package files
COPY package.json package-lock.json ./

# Install dependencies and clean cache
RUN npm ci --omit=dev && npm cache clean --force

# Copy source code
COPY src ./src

# Add non-root user for security
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nodejs -u 1001 && \
    chown -R nodejs:nodejs /app

USER nodejs

# Expose gateway port
EXPOSE 8080

# Health check to ensure gateway responsiveness
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD node -e "require('http').get('http://localhost:8080/health', (r) => { process.exit(r.statusCode === 200 ? 0 : 1); }).on('error', () => process.exit(1));"

# Default command
CMD ["node", "src/server.mjs"]
