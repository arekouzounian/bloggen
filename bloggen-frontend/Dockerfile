# https://github.com/vercel/next.js/blob/canary/examples/with-docker/Dockerfile

FROM node:22-alpine AS base 

FROM base AS deps 
RUN apk add --no-cache libc6-compat 

WORKDIR /app
COPY package*.json ./
RUN npm ci 

# Build Stage 
FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .

# to avoid telemetry
ENV NEXT_TELEMETRY_DISABLED 1
RUN npm run build

# Run Stage 
FROM base as runner
WORKDIR /app 
ENV NODE_ENV production
# just in case it wasn't disabled during build 
ENV NEXT_TELEMETRY_DISABLED 1 

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
COPY --from=builder /app/public ./public

RUN mkdir .next
RUN chown nextjs:nodejs .next

COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static


USER nextjs
EXPOSE 3000
ENV PORT 3000

# server.js is created by next build from the standalone output
# https://nextjs.org/docs/pages/api-reference/next-config-js/output
CMD HOSTNAME="0.0.0.0" node server.js
