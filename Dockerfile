FROM rustlang/rust:nightly
LABEL version = "0.1" \
        description = "poa" \
        vendor = "cc"
SHELL ["/bin/bash", "-c"]
WORKDIR /home/PoaDemo/
COPY ./Cargo.toml /home/PoaDemo/
COPY ./src/* /home/PoaDemo/src/
RUN cargo build
ENTRYPOINT ./target/debug/PoaDemo -u=user -p=password -n=172.33.0.1:4222
EXPOSE 4222