FROM rust:slim
RUN apt-get update && apt-get install -y cmake libclang-dev llvm-dev libssl-dev clang
