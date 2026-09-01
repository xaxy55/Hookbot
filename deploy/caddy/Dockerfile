# Caddy with the Cloudflare DNS plugin, needed for the DNS-01 ACME challenge.
# The stock caddy image has no DNS providers compiled in.
FROM caddy:2-builder AS builder
RUN xcaddy build --with github.com/caddy-dns/cloudflare

FROM caddy:2
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
