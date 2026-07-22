Using set of services:

- Nuxt <https://github.com/ulcuber/nuxt-laravel-auth>
    - Frontend Client
    - NodeJS Nuxt Server
    - Main nginx host `slots.localhost`
- Nginx
    - **Proxy** for `slots.localhost` and `id.slots.localhost`
- Laravel Passport ID <https://github.com/ulcuber/laravel-passport-id>
    - **Authorization Server**
    - Host `id.slots.localhost`
    - DB `slots-id`
- Rust Laravel Passport Introspector (this) <https://github.com/ulcuber/laravel-passport-introspection>
    - **Authorization Server**
    - Introspection Server
    - Introspection **Proxy** (no balancer) for entry setup
    - Introspection Mono-Proxy for using within service **pod**
    - DB `slots-id`
- Rust Slots Service <https://github.com/ulcuber/axum-slots>
    - **Resource Server**
    - Host location `slots.localhost/api/slots/*`
    - DB `slots`
    - First nginx failover for `/api/slots/`
- Laravel Slots Service <https://github.com/ulcuber/laravel-slots>
    - **Resource Server**
    - Host location `slots.localhost/api/slots/*`
    - DB `slots`
    - Second nginx failover for `/api/slots/` with Octane on `localhost:8006`
    - Third nginx failover for `/api/slots/` with PHP-FPM

```env
DATABASE_URL=.../slots-id
... other defaults
```

**Laravel**

No `DB_SOCKET=/var/run/mysqld/mysqld.sock`, no `REDIS_SOCKET=/run/valkey.sock` for closer to 12factor

```bash
php artisan db:seed
php artisan optimize
```

Service uses 10 seconds cache with atomic locks

```bash
./wrk/init_mysql.sh
cargo run --bin wrk_access_tokens_factory --features=write-tokens
```

**Hardware**

- `AMD Ryzen 5 4500U with Radeon Graphics`
- `2 x 8 Gib DDR4 2667 MT/s`

# Self

## No cache

```env
TOKEN_CACHE_SIZE=2
TOKEN_CACHE_TTL=0
WRK_ROTATE_EVERY=10
```

```bash
cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens -http
```

```
Running 1m test @ http://localhost:8080/introspect-http
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    23.43ms    7.95ms 321.50ms   97.56%
    Req/Sec     1.43k   110.81     3.38k    84.04%
  1026923 requests in 1.00m, 145.81MB read
Requests/sec:  17087.48
Transfer/sec:      2.43MB
```

## Cached worst case (cache not used)

```env
TOKEN_CACHE_SIZE=1000
TOKEN_CACHE_TTL=60
WRK_ROTATE_EVERY=1
```

```bash
cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens -http
```

```
Running 1m test @ http://localhost:8080/introspect-http
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    23.14ms    6.84ms 323.56ms   94.82%
    Req/Sec     1.44k   106.62     3.31k    83.46%
  1036478 requests in 1.00m, 147.17MB read
Requests/sec:  17246.64
Transfer/sec:      2.45MB
```

## 10 requests with same token

```env
TOKEN_CACHE_SIZE=1000
TOKEN_CACHE_TTL=60
WRK_ROTATE_EVERY=10
```

```bash
cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens -http
```

```
Running 1m test @ http://localhost:8080/introspect-http
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.56ms    1.27ms  11.93ms   65.89%
    Req/Sec    13.00k   671.90    26.17k    85.22%
  9331461 requests in 1.00m, 1.29GB read
Requests/sec: 155281.90
Transfer/sec:     22.05MB
```

## 20M requests with same token (same user DDOS, max cache usage)

```env
TOKEN_CACHE_SIZE=1000
TOKEN_CACHE_TTL=60
WRK_ROTATE_EVERY=20000000
```

```bash
cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens -http
```

```
Running 1m test @ http://localhost:8080/introspect-http
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.75ms    0.88ms  10.48ms   69.80%
    Req/Sec    19.03k     1.43k   51.13k    77.26%
  13658635 requests in 1.00m, 1.87GB read
Requests/sec: 227290.25
Transfer/sec:     31.86MB
```

# Laravel Octane app no auth

```env
APP_URL=http://localhost:8006/slots/availability
```

Pre-check: `./scripts/curl_env_app.sh`

```bash
# app
php artisan octane:start --port=8006

./wrk/wrk_env_app.sh rotate_app_users
```

```
Running 1m test @ http://localhost:8006/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   156.19ms   86.58ms   1.15s    88.95%
    Req/Sec   222.53     64.42   333.00     68.50%
  159731 requests in 1.00m, 331.89MB read
Requests/sec:   2657.78
Transfer/sec:      5.52MB
```

# Laravel Octane Authorization Server (Passport ID)

Does PHP side cryptography + Token and Client DB checks

Requires existing user for `Laravel\Passport\Guards\TokenGuard`:

```php
<?php

$user = $this->provider->retrieveById($oauthUserId);
```

So user route will receive the same user from guard

Because of that use Laravel seeder after rust tokens factory to generate users for existing tokens:

```bash
php artisan db:seed
```

```env
PROXY_URL=http://localhost:8007/api/oidc-user
```


Pre-check: `./scripts/curl_env_proxy.sh`

```bash
# app
php artisan octane:start --port=8007

./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://localhost:8007/api/oidc-user
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   281.65ms   50.80ms 489.68ms   88.81%
    Req/Sec   118.09     52.87   343.00     65.86%
  Latency Distribution
     50%  265.78ms
     75%  282.58ms
     90%  367.43ms
     99%  452.22ms
  84306 requests in 1.00m, 20.33MB read
  Non-2xx or 3xx responses: 107
Requests/sec:   1402.81
Transfer/sec:    346.40KB
```

# Rust app no auth (valkey cache, TCP)

```env
APP_URL=http://localhost:8081/slots/availability
```

Pre-check: `./scripts/curl_env_app.sh`

```bash
# app
cargo run --release

./wrk/wrk_env_app.sh rotate_app_users
```

```
Running 1m test @ http://localhost:8081/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     3.89ms    1.04ms 206.53ms   92.11%
    Req/Sec     8.54k   485.85    17.34k    81.09%
  6122350 requests in 1.00m, 5.67GB read
Requests/sec: 101868.68
Transfer/sec:     96.66MB
```

# Rust app no auth, no connections

```env
APP_URL=http://localhost:8081/slots/hello
```

Pre-check: `./scripts/curl_env_app.sh`

```bash
# app
cargo run --release

./wrk/wrk_env_app.sh rotate_app_users
```

```
Running 1m test @ http://localhost:8081/slots/hello
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.48ms    0.85ms  11.34ms   70.75%
    Req/Sec    22.16k     2.08k   39.07k    79.17%
  15871527 requests in 1.00m, 1.92GB read
Requests/sec: 264206.39
Transfer/sec:     32.76MB
```

# Nginx

```env
PROXY_URL=http://slots.localhost/api/slots/availability
TOKEN_CACHE_SIZE=1000
TOKEN_CACHE_TTL=60
WRK_ROTATE_EVERY=10
```

## HTTP over TCP

`/etc/nginx/conf.d/slots.conf`:

```conf
proxy_pass http://slots-introspector/introspect-http;
```

### PHP-FPM

`/etc/nginx/conf.d/slots.conf`:

```conf
try_files "" @fpm;
```

`/etc/php/php-fpm.d/www.conf`:

```conf
pm.max_children = 800
pm.start_servers = 400
pm.min_spare_servers = 400
pm.max_spare_servers = 800
```

```bash
cargo run --release
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://slots.localhost/api/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   355.82ms  179.47ms   2.00s    82.40%
    Req/Sec    92.90     27.81   313.00     69.34%
  Latency Distribution
     50%  358.08ms
     75%  458.97ms
     90%  510.31ms
     99%    1.02s
  66857 requests in 1.00m, 141.29MB read
  Socket errors: connect 0, read 0, write 0, timeout 270
  Non-2xx or 3xx responses: 275
Requests/sec:   1112.48
Transfer/sec:      2.35MB
```

### Octane

`/etc/nginx/conf.d/slots.conf`:

```conf
try_files "" @octane;
```

```bash
# resource server app
php artisan octane:start --port=8006

cargo run --release
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://slots.localhost/api/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   175.64ms   79.63ms 961.20ms   87.95%
    Req/Sec   193.70     40.78   323.00     71.21%
  Latency Distribution
     50%  151.77ms
     75%  175.17ms
     90%  268.02ms
     99%  560.35ms
  139238 requests in 1.00m, 290.88MB read
  Non-2xx or 3xx responses: 227
Requests/sec:   2316.99
Transfer/sec:      4.84MB
```

### Rust Axum service

`/etc/nginx/conf.d/slots.conf`:

```conf
try_files "" @rust;
```

```env
PROXY_URL=http://slots.localhost/api/slots/availability
```

```bash
# resource server app
cargo run --release

cargo run --release
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://slots.localhost/api/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   184.05ms  302.39ms   1.28s    81.72%
    Req/Sec     2.10k   485.08     5.48k    70.65%
  Latency Distribution
     50%    9.00ms
     75%  279.99ms
     90%  740.86ms
     99%    1.02s
  1508967 requests in 1.00m, 1.49GB read
  Socket errors: connect 0, read 0, write 0, timeout 37
Requests/sec:  25108.84
Transfer/sec:     25.38MB
```

## HTTP over sockets

Not measured

# Introspection Proxy

## Rust Axum service

```env
PROXY_URL=http://localhost:8080/api/slots/availability
```

```bash
# resource server app
cargo run --release

cargo run --release --bin proxy --features proxy
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://localhost:8080/api/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     7.86ms    2.80ms  46.27ms   70.52%
    Req/Sec     4.22k   259.22    10.20k    78.01%
  Latency Distribution
     50%    7.57ms
     75%    9.50ms
     90%   11.48ms
     99%   15.49ms
  3034045 requests in 1.00m, 2.81GB read
Requests/sec:  50492.17
Transfer/sec:     47.91MB
```

# Introspection Mono-Proxy

## Rust Axum service

```env
MONO_PROXY_TARGET=http://localhost:8081
PROXY_URL=http://localhost:8080/slots/availability
```

```bash
# resource server app
cargo run --release

cargo run --release --bin mono_proxy --features proxy
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://localhost:8080/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     7.93ms    2.81ms  48.37ms   69.92%
    Req/Sec     4.18k   244.09     8.90k    75.53%
  Latency Distribution
     50%    7.64ms
     75%    9.61ms
     90%   11.62ms
     99%   15.49ms
  3002729 requests in 1.00m, 2.78GB read
Requests/sec:  49969.63
Transfer/sec:     47.42MB
```

## Octane service

```env
PROXY_URL=http://localhost:8080/slots/availability
MONO_PROXY_TARGET=http://127.0.0.1:8006
```

```bash
# resource server app
php artisan octane:start --port=8006

cargo run --release --bin mono_proxy --features proxy
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://localhost:8080/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   157.78ms   80.48ms   1.16s    88.40%
    Req/Sec   217.72     44.35   333.00     74.83%
  Latency Distribution
     50%  136.22ms
     75%  144.77ms
     90%  249.34ms
     99%  575.27ms
  156431 requests in 1.00m, 324.62MB read
  Non-2xx or 3xx responses: 227
Requests/sec:   2603.20
Transfer/sec:      5.40MB
```

# socketsd

```bash
cargo run --release --bin introspection_socketsd --features sockets
cargo run --release --example sockets_benchmark
```

```
Total requests: 1000000
Success: 1000000
Errors: 0
Time: 57.72463609s
RPS: 17324
Latency p50: 22.84ms
Latency p90: 28.15ms
Latency p99: 32.70ms
```

# Rust slots service with introspection library

```env
# monolithic
PROXY_URL=http://localhost:8081/slots/availability
```

Slots `.env`:

```env
INTERNAL_INTROSPECTOR=true
```

Fill `.env.introspector` in resource server

Pre-check: `./scripts/curl_env_proxy.sh`

```bash
# app
cargo run --release

./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://localhost:8081/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     4.87ms    1.33ms  33.81ms   77.50%
    Req/Sec     6.83k   324.22    15.08k    83.77%
  Latency Distribution
     50%    4.74ms
     75%    5.53ms
     90%    6.32ms
     99%    8.05ms
  4903203 requests in 1.00m, 4.54GB read
Requests/sec:  81586.86
Transfer/sec:     77.42MB
```
