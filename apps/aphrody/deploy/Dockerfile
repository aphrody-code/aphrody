# syntax=docker/dockerfile:1
# Build the aphrody wheel, then a slim runtime that serves a static/SPA site.
# Build context = the apps/aphrody package:
#   docker build -f apps/aphrody/deploy/Dockerfile -t aphrody-serve apps/aphrody
# Run (mount your built React dist at /srv/site):
#   docker run --rm -p 8080:8080 -v /path/to/dist:/srv/site:ro aphrody-serve
# SPDX-License-Identifier: Apache-2.0

FROM python:3.12-slim AS build
WORKDIR /src
COPY . /src
RUN pip install --no-cache-dir build && python -m build --wheel --outdir /dist

FROM python:3.12-slim
LABEL org.opencontainers.image.title="aphrody-serve" \
      org.opencontainers.image.licenses="Apache-2.0"
RUN useradd --system --create-home aphrody
COPY --from=build /dist/*.whl /tmp/
RUN pip install --no-cache-dir /tmp/*.whl && rm -f /tmp/*.whl
USER aphrody
ENV APHRODY_SITE_ROOT=/srv/site \
    APHRODY_HOST=0.0.0.0 \
    APHRODY_PORT=8080
EXPOSE 8080
# --cache preloads the site into RAM. To front a Rust backend instead, append
# e.g. --proxy http://backend:3000 --proxy-prefix /api
CMD ["sh", "-c", "aphrody serve \"$APHRODY_SITE_ROOT\" --host \"$APHRODY_HOST\" --port \"$APHRODY_PORT\" --cache"]
