"""Compile settings only: never persist Cargo registry credentials or toolchain URLs."""
import re


def capture(environ):
    exact = {'CC','CXX','AR','CFLAGS','CXXFLAGS','CPPFLAGS','LDFLAGS','TARGET','HOST',
             'RUSTFLAGS','RUSTDOCFLAGS','RUSTC','RUSTDOC','RUSTC_WRAPPER',
             'RUSTC_WORKSPACE_WRAPPER','CARGO_ENCODED_RUSTFLAGS','CARGO_INCREMENTAL',
             'CARGO_BUILD_TARGET','CARGO_BUILD_RUSTFLAGS','CARGO_TARGET_DIR'}
    prefixes = ('CC_','CXX_','AR_','CFLAGS_','CXXFLAGS_','CPPFLAGS_','LDFLAGS_',
                'CARGO_TARGET_','CARGO_PROFILE_','SQLITE','LIBSQLITE','PKG_CONFIG')
    return {k:v for k,v in environ.items()
            if (k in exact or k.startswith(prefixes))
            and not re.search('token|password|secret|credential|authorization|cookie',k,re.I)}
