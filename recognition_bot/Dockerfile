FROM rust:1.32.0

WORKDIR /app
COPY . .

RUN apt-get update
RUN apt-get install -y libav-tools
RUN cargo install --path .

CMD ["recognition_bot"]