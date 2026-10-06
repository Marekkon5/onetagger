FROM node:22.23.3-alpine AS build
WORKDIR /app
RUN npm install --global pnpm@12.6.0
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/domain/package.json ./packages/domain/package.json
COPY worker/package.json ./worker/package.json
RUN pnpm install --frozen-lockfile --filter @syco23/import-worker...
COPY packages/domain ./packages/domain
COPY worker ./worker
RUN pnpm --dir packages/domain build
RUN pnpm --dir worker build

FROM node:22.23.3-alpine AS runtime
WORKDIR /app
RUN apk add --no-cache ffmpeg=7.1.1-r0 && ffmpeg -version | head -1
ENV NODE_ENV=production
ENV IMPORT_STORAGE_DRIVER=local
ENV IMPORT_STORAGE_LOCAL_ROOT=/app/data/imports
COPY --from=build /app/package.json ./package.json
COPY --from=build /app/pnpm-workspace.yaml ./pnpm-workspace.yaml
COPY --from=build /app/pnpm-lock.yaml ./pnpm-lock.yaml
COPY --from=build /app/packages/domain/package.json ./packages/domain/package.json
COPY --from=build --chown=node:node /app/packages/domain/dist ./packages/domain/dist
COPY --from=build /app/worker/package.json ./worker/package.json
COPY --from=build --chown=node:node /app/worker/dist ./worker/dist
RUN pnpm install --prod --frozen-lockfile --filter @syco23/import-worker...
RUN mkdir -p /app/data/imports /tmp/syco23-worker && chown -R node:node /app/data /tmp/syco23-worker
USER node
CMD ["node", "worker/dist/index.js"]
