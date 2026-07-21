# Benchmarking

## Common

Update root `.env` from `.env.wrk`

```bash
# once
./wrk/init_mysql.sh
cargo run --bin wrk_access_tokens_factory --features=write-tokens

cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens
./wrk/wrk_env.sh rotate_valid_tokens -json
./wrk/wrk_env.sh rotate_valid_tokens -http
```

For large amount of tokens

```bash
mv wrk/tokens_RS256.txt wrk/tokens_RS256_all.txt
shuf -n 1000 wrk/tokens_RS256_all.txt > wrk/tokens_RS256.txt
```

Init uses `drop table if exists`. Factory generates real tokens into `wrk/tokens_<ALG>.txt`. Some scripts use random tokens from there

## Alternative launch options:

- `/usr/bin/time -v ./target/release/introspection_server`
- `perf stat -e cycles,instructions,cache-references,cache-misses,LLC-loads,LLC-load-misses ./target/release/introspection_server`
- `./wrk/wrk_env.sh && pkill -INT -f introspection_server`

## Profiling

```bash
perf record -F 999 -g -- ./target/release/introspection_server
./wrk/wrk_env.sh rotate_valid_tokens -http && pkill -INT -f introspection_server
perf report
```

### Unix sockets introspector

```bash
cargo run --release --bin introspection_socketsd --features sockets
ulimit -n 4096 && cargo run --release --example sockets_benchmark

# more like measuring socat bridge
cargo run --release --bin introspection_http_socketsd --features sockets
socat TCP-LISTEN:8080,fork,reuseaddr UNIX-CONNECT:/tmp/introspector.sock
ulimit -n 4096 && ./wrk/wrk_env.sh rotate_valid_tokens -http
```

### Proxy server

`/etc/php/php-fpm.d/www.conf`

```conf
pm.max_children = 400
pm.start_servers = 300
pm.min_spare_servers = 200
pm.max_spare_servers = 400
```

For Laravel

```bash
php artisan optimize
```

```bash
# configure test db in service then fill with factory
cargo run --bin wrk_access_tokens_factory --features=write-tokens

cargo run --release --bin introspection_http_socketsd --features sockets
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```
