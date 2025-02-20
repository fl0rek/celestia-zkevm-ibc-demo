FROM docker.io/golang:1.23-alpine3.20

RUN apk add --no-cache make jq bash git ncurses build-base curl

RUN git clone https://github.com/rollkit/beacon-kit.git /beacond &&\
        cd /beacond &&\
        git checkout docker-compose-rollkit &&\
        go clean -modcache &&\
        make build

