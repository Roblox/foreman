#!/usr/bin/env bash

set -x
set -e

OPENSSL_VER=${OPENSSL_VER:-openssl-1.1.1n}
OPENSSL_DST=${PWD}/${OPENSSL_VER}-install

if [[ ! -d ${OPENSSL_DST} ]]; then
    curl -O https://www.openssl.org/source/${OPENSSL_VER}.tar.gz
    tar xzf ${OPENSSL_VER}.tar.gz
    cd ${OPENSSL_VER}
    ./Configure no-shared enable-rc5 zlib darwin64-arm64-cc --prefix=${OPENSSL_DST} --openssldir=${OPENSSL_DST}
    make -j2
    make install
    cd -
fi

export OPENSSL_DIR=${OPENSSL_DST}