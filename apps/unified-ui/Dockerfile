# Unified UI production image
FROM node:20-alpine AS builder

WORKDIR /app

COPY package.json pnpm-lock.yaml* package-lock.json* yarn.lock* ./

RUN corepack enable || true
RUN [ -f pnpm-lock.yaml ] && pnpm install --frozen-lockfile || \
    ([ -f package-lock.json ] && npm ci || \
     ([ -f yarn.lock ] && yarn install || npm install))

COPY . .

ENV NODE_ENV=production

RUN npm run build || pnpm build || yarn build

FROM node:20-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000

COPY --from=builder /app ./

EXPOSE 3000

CMD ["npm", "run", "start"]
