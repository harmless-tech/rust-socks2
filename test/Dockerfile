FROM debian:latest

ARG CONFIG=./test/dante_no_auth.conf

RUN apt update
RUN apt install -y dante-server

RUN useradd -m testuser
RUN echo 'testuser:testpass' | chpasswd

COPY ${CONFIG} /etc/danted.conf

ENTRYPOINT ["/usr/sbin/danted"]
