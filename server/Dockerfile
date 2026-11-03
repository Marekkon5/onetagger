FROM node:22.23.3-alpine AS build
WORKDIR /app

RUN npm install --global pnpm@12.8.1

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/domain/package.json ./packages/domain/package.json
COPY server/package.json ./server/package.json
RUN pnpm install --frozen-lockfile --filter @syco23/mixsets-server...

COPY packages/domain ./packages/domain
COPY server ./server
RUN pnpm --dir packages/domain build
RUN pnpm --dir server build
RUN pnpm --filter @syco23/mixsets-server deploy --prod /runtime

FROM node:22.23.3-alpine AS runtime
WORKDIR /app
ARG VCS_REF=unknown
LABEL org.opencontainers.image.revision=${VCS_REF}

RUN apk add --no-cache ffmpeg

ENV NODE_ENV=production
ENV PORT=8787
ENV CATALOG_DB_PATH=/app/data/catalog.sqlite

COPY --from=build --chown=node:node /runtime/node_modules ./server/node_modules
COPY --from=build /runtime/package.json ./server/package.json
COPY --from=build --chown=node:node /app/server/dist ./server/dist
RUN mkdir -p /app/data && chown node:node /app/data

USER node
EXPOSE 8787

CMD ["node", "server/dist/index.js"]
