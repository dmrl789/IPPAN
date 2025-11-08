FROM node:20-alpine

ENV NODE_ENV=production
ENV PORT=8080

WORKDIR /app

COPY package.json package-lock.json* ./

RUN npm ci --omit=dev && npm cache clean --force

COPY src ./src

USER node

EXPOSE 8080

CMD ["node", "src/server.mjs"]
