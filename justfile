pwd := `pwd`

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
    docker run -d --rm --name dani1 -p 1084:1084/tcp -p 1084:1084/udp dante-test-img:no-auth
    docker run -d --rm --name dani2 -p 1085:1085/tcp -p 1085:1085/udp dante-test-img:password

test:
    cargo test

testr:
    cargo testr

test-teardown:
    docker stop dani1 || true
    docker stop dani2 || true

hack:
    docker run -it --rm --pull=always \
    --mount type=bind,source={{pwd}},target=/project \
    --mount type=bind,source=$HOME/.cargo/registry,target=/usr/local/cargo/registry \
    --entrypoint=/bin/bash \
    ghcr.io/cargo-prebuilt/ink-cross:stable-native \
    -c 'cargo prebuilt --ci cargo-hack && cargo hack check --each-feature --no-dev-deps --verbose --workspace && cargo hack check --feature-powerse --no-dev-deps --verbose --workspace'

msrv:
    docker run -it --rm --pull=always \
    --mount type=bind,source={{pwd}},target=/project \
    --mount type=bind,source=$HOME/.cargo/registry,target=/usr/local/cargo/registry \
    --entrypoint=/bin/bash \
    ghcr.io/cargo-prebuilt/ink-cross:stable-native \
    -c 'cargo prebuilt --ci cargo-msrv && cargo msrv find -- cargo check --verbose'

msrv-verify:
    docker run -it --rm --pull=always \
    --mount type=bind,source={{pwd}},target=/project \
    --mount type=bind,source=$HOME/.cargo/registry,target=/usr/local/cargo/registry \
    --entrypoint=/bin/bash \
    ghcr.io/cargo-prebuilt/ink-cross:stable-native \
    -c 'cargo prebuilt --ci cargo-msrv && cargo msrv verify -- cargo check --verbose --release'
