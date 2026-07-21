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
    Latency     3.05ms    4.23ms 273.28ms   98.76%
    Req/Sec    11.41k     0.89k   20.26k    91.31%
  8181577 requests in 1.00m, 1.13GB read
Requests/sec: 136144.49
Transfer/sec:     19.33MB
```

## 10M requests with same token (same user DDOS, max cache usage)

```env
TOKEN_CACHE_SIZE=1000
TOKEN_CACHE_TTL=60
WRK_ROTATE_EVERY=10000000
```

```bash
cargo run --release
./wrk/wrk_env.sh rotate_valid_tokens -http
```

```
Running 1m test @ http://localhost:8080/introspect-http
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.79ms    1.01ms  99.62ms   75.14%
    Req/Sec    18.56k     1.39k   55.96k    76.32%
  13325560 requests in 1.00m, 1.82GB read
Requests/sec: 221733.37
Transfer/sec:     31.08MB
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
pm.max_children = 400
pm.start_servers = 300
pm.min_spare_servers = 200
pm.max_spare_servers = 400
```

```bash
cargo run --release
./wrk/wrk_env_proxy.sh rotate_valid_tokens_proxy
```

```
Running 1m test @ http://slots.localhost/api/slots/availability
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   360.55ms  185.85ms   2.00s    68.00%
    Req/Sec    91.50     31.62   277.00     74.51%
  65944 requests in 1.00m, 139.56MB read
  Socket errors: connect 0, read 0, write 0, timeout 59
  Non-2xx or 3xx responses: 149
Requests/sec:   1097.28
Transfer/sec:      2.32MB
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
    Latency   183.11ms   73.14ms   1.06s    86.75%
    Req/Sec   183.75     40.06   320.00     72.29%
  131995 requests in 1.00m, 275.73MB read
  Non-2xx or 3xx responses: 227
Requests/sec:   2196.46
Transfer/sec:      4.59MB
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
    Latency    44.12ms    7.10ms 382.34ms   99.25%
    Req/Sec   754.15     49.10     1.08k    87.80%
  541711 requests in 1.00m, 547.61MB read
Requests/sec:   9014.97
Transfer/sec:      9.11MB
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
    Latency     9.46ms    5.25ms 234.53ms   92.67%
    Req/Sec     3.57k   270.13     6.08k    85.81%
  2565721 requests in 1.00m, 2.38GB read
Requests/sec:  42699.23
Transfer/sec:     40.52MB
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
    Latency     9.16ms    3.81ms 176.23ms   82.53%
    Req/Sec     3.65k   264.92     8.44k    86.14%
  2622139 requests in 1.00m, 2.43GB read
Requests/sec:  43633.18
Transfer/sec:     41.40MB
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
    Latency   164.64ms   80.04ms   1.17s    88.04%
    Req/Sec   208.01     41.59   333.00     71.15%
  149382 requests in 1.00m, 309.97MB read
  Non-2xx or 3xx responses: 227
Requests/sec:   2485.78
Transfer/sec:      5.16MB
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
    Latency     6.25ms    2.32ms 170.72ms   94.19%
    Req/Sec     5.36k   535.32    10.84k    75.33%
  3845985 requests in 1.00m, 3.56GB read
Requests/sec:  64001.39
Transfer/sec:     60.73MB
```
