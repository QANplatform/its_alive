FROM rustlang/rust:nightly
LABEL version = "0.1" \
        description = "poa" \
        vendor = "cc"
SHELL ["/bin/bash", "-c"]
WORKDIR /home/PoaDemo/
RUN apt-get update -y 
RUN apt-get install -y libclang-dev llvm clang
# Install Rocksdb
RUN cd /tmp && \
    git clone https://github.com/facebook/rocksdb.git && \
    cd rocksdb && \
    make shared_lib && \
    mkdir -p /usr/local/rocksdb/lib && \
    mkdir /usr/local/rocksdb/include && \
    cp librocksdb.so* /usr/local/rocksdb/lib && \
    cp /usr/local/rocksdb/lib/librocksdb.so* /usr/lib/ && \
    cp -r include /usr/local/rocksdb/ && \
    cp -r include/* /usr/include/ && \
    rm -R /tmp/rocksdb/
COPY ./Cargo.toml /home/PoaDemo/
COPY ./src/* /home/PoaDemo/src/
COPY ./fence /home/PoaDemo/fence/
RUN cargo build
ENTRYPOINT ./target/debug/poa_demo -u=user -p=password -n=172.33.0.1:4222
EXPOSE 4222
EXPOSE 8000