FROM rust:1.78 AS base 
WORKDIR /usr/src/server 
COPY . .

# maybe get from json file but default is 2222
EXPOSE 2222

RUN cargo install --path .

CMD ["server", "docker.json"]
