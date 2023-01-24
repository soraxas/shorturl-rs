FROM rust:1.61.0
 
WORKDIR /shorturl-src
COPY . .
 
RUN cargo install --path .
 
CMD ["short-url"]
