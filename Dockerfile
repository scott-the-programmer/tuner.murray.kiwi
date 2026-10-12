# Build dist with `trunk build --release --locked` before building this image.
FROM nginx:stable-alpine
LABEL org.opencontainers.image.source="https://github.com/scott-the-programmer/tuner.murray.kiwi"
LABEL org.opencontainers.image.description="Guitar and ukulele tuner built with Rust and Dioxus"
LABEL org.opencontainers.image.licenses="MIT"
COPY dist/ /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1/healthz || exit 1
CMD ["nginx", "-g", "daemon off;"]
