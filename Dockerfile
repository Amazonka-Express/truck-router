FROM rust:1.87

RUN mkdir /app
WORKDIR /app

COPY . .

RUN curl -L "https://drive.google.com/uc?export=download&id=1RbtK-rm6O4AJ3kdUCXnaVx_Udvfskdze" -o nodes.bin
RUN chmod +x gdrive-download.sh && \
    ./gdrive-download.sh "https://drive.google.com/uc?export=download&id=18xARrgFcpu0IHQ1yoMFXqrs3xWV-3VtY" ch.bin

RUN apt-get update
RUN apt-get install -y protobuf-compiler

RUN cargo build --release

EXPOSE 50051

CMD ["./target/release/truck-router"]
