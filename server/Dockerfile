FROM node:22.23.3-alpine AS build
WORKDIR /app

RUN npm install --global pnpm@12.6.0

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/domain/package.json ./packages/domain/package.json
COPY server/package.json ./server/package.json
RUN pnpm install --frozen-lockfile --filter @syco23/mixsets-server...

COPY packages/domain ./packages/domain
COPY server ./server
RUN pnpm --dir packages/domain build
RUN pnpm --dir server build

FROM node:22.23.3-alpine AS runtime
WORKDIR /app

RUN apk add --no-cache ffmpeg

ENV NODE_ENV=production
ENV PORT=8787
ENV CATALOG_DB_PATH=/app/data/catalog.sqlite

COPY --from=build /app/package.json ./package.json
COPY --from=build /app/pnpm-workspace.yaml ./pnpm-workspace.yaml
COPY --from=build /app/pnpm-lock.yaml ./pnpm-lock.yaml
COPY --from=build /app/packages/domain/package.json ./packages/domain/package.json
COPY --from=build --chown=node:node /app/packages/domain/dist ./packages/domain/dist
COPY --from=build --chown=node:node /app/server/dist/catalog/migrations ./server/dist/catalog/migrations
COPY --from=build /app/server/package.json ./server/package.json
RUN pnpm install --prod --frozen-lockfile --filter @syco23/mixsets-server...
COPY --from=build --chown=node:node /app/server/dist ./server/dist
RUN mkdir -p /app/data && chown node:node /app/data

USER node
EXPOSE 8787

CMD ["node", "server/dist/index.js"]
