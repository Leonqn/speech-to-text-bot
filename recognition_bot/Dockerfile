FROM rust

WORKDIR /app
COPY . .

RUN apt-get update
RUN apt-get install -y ffmpeg
RUN cargo install --path .

CMD ["recognition_bot"]