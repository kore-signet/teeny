FROM tiangolo/uvicorn-gunicorn:python3.8-slim

WORKDIR /

RUN apt-get update && apt-get install liblmdb-dev --no-install-recommends -y

COPY ./app/requirements.txt /
RUN python3 -m pip install -r requirements.txt

COPY ./app /app
