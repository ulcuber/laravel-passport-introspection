#!/bin/bash

MYSQL_BIN="mysql"
if [[ $(command -v mariadb) ]]; then
        MYSQL_BIN="mariadb"
fi

"$MYSQL_BIN" -se "SELECT id, user, host, db, command, time, state FROM information_schema.processlist ORDER BY time DESC LIMIT 10;"

echo

"$MYSQL_BIN" -Nse "SHOW STATUS;" |\
        grep -P '^(Aborted_clients|Aborted_connects|Threads_connected|Max_used_connections|Connection_errors_.*)\s'

"$MYSQL_BIN" -Nse "SHOW VARIABLES;" |\
        grep -P '^(max_connections|max_user_connections)\s'
