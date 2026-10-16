FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8000

# trivy DS-0002: never run as root inside the container.
RUN useradd --uid 10001 --no-create-home rise
USER 10001

# trivy DS-0026: container-level healthcheck against the API /health route.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD python -c 'import os,sys,urllib.request as u; sys.exit(0 if u.urlopen("http://127.0.0.1:%s/health" % os.environ.get("PORT","8000"), timeout=2).status==200 else 1)'

CMD ["sh", "-c", "uvicorn app:app --host 0.0.0.0 --port ${PORT:-8000}"]
