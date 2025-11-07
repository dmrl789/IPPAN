FROM node:20-alpine AS deps

WORKDIR /app

ENV NEXT_TELEMETRY_DISABLED=1 \
    NODE_ENV=production

COPY package.json package-lock.json* ./
RUN npm ci

COPY . .
RUN npm run build

FROM node:20-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3001

COPY --from=deps /app/package.json ./package.json
COPY --from=deps /app/package-lock.json ./package-lock.json
COPY --from=deps /app/next.config.js ./next.config.js
COPY --from=deps /app/public ./public
COPY --from=deps /app/.next ./.next

RUN npm ci --omit=dev

EXPOSE 3001

CMD ["npm", "run", "start"]
