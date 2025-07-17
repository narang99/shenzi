FROM quay.io/pypa/manylinux_2_28_x86_64

RUN yum install openssl openssl-devel  -y
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

COPY python/shenzi /python/shenzi
COPY crates /crates

ENV WHEEL_PLATFORM=manylinux_2_31_x86_64
