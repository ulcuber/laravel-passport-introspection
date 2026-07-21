#!/bin/bash

set -a
source .env
set +a

MYSQL_BIN="mysql"
if [[ $(command -v mariadb) ]]; then
    MYSQL_BIN="mariadb"
fi

echo "Initializing database: $DATABASE_URL"

DB_USER=$(echo "$DATABASE_URL" | sed -n 's/mysql:\/\/\([^:]*\):.*/\1/p')
DB_PASS=$(echo "$DATABASE_URL" | sed -n 's/mysql:\/\/[^:]*:\([^@]*\)@.*/\1/p')
DB_HOST=$(echo "$DATABASE_URL" | sed -n 's/mysql:\/\/[^@]*@\([^:]*\):.*/\1/p')
DB_PORT=$(echo "$DATABASE_URL" | sed -n 's/mysql:\/\/[^@]*@[^:]*:\([0-9]*\)\/.*/\1/p')
DB_NAME=$(echo "$DATABASE_URL" | sed -n 's/.*\/\([^?]*\)/\1/p')

"$MYSQL_BIN" -u "$DB_USER" -p"$DB_PASS" -h "$DB_HOST" -P "$DB_PORT" -e "CREATE DATABASE IF NOT EXISTS $DB_NAME;" || exit 1

echo "Importing schema..."

# source is safer for mb encodings than bash pipes
"$MYSQL_BIN" -u "$DB_USER" -p"$DB_PASS" -h "$DB_HOST" -P "$DB_PORT" -e "source wrk/mysql_schema.sql" "$DB_NAME" || exit 1

echo "Database initialized!"
