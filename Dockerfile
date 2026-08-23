FROM node:22-bookworm-slim AS build

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

FROM node:22-bookworm-slim

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY wrangler.toml ./
COPY migrations ./migrations/
COPY docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

COPY --from=build /app/.svelte-kit ./.svelte-kit

EXPOSE 8788

ENV BETTER_AUTH_SECRET=dev-secret-change-in-production-min-32-chars!!
ENV BETTER_AUTH_URL=http://localhost:8788

ENTRYPOINT ["/docker-entrypoint.sh"]
