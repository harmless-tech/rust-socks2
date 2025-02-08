default:
    just -l

fmt:
    cargo +nightly fmt

check:
    cargo +nightly fmt --check
    cargo clippy --all-targets --workspace -- -D warnings
    cargo clippy --all-targets --workspace --release -- -D warnings
    cargo deny check

check-nightly:
    cargo +nightly fmt --check
    cargo +nightly clippy --all-targets --workspace -- -D warnings
    cargo +nightly clippy --all-targets --workspace --release -- -D warnings
    cargo +nightly deny check

test-setup: test-teardown
    docker build -t dante-test-img:no-auth -f ./test/Dockerfile \
        --build-arg CONFIG=./test/dante_no_auth.conf .
    docker build -t dante-test-img:password -f ./test/Dockerfile \
            --build-arg CONFIG=./test/dante_password.conf .
    docker run -d --rm --name dani1 -p 1084:1084 -p 15410-15413:15410-15413/udp dante-test-img:no-auth
    docker run -d --rm --name dani2 -p 1085:1085 -p 15414-15415:15414-15415/udp dante-test-img:password

test:
    cargo test

testr:
    cargo testr

test-teardown:
    docker stop dani1 || true
    docker stop dani2 || true
