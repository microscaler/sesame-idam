#!/usr/bin/env bash

# This script generates SeaORM models from the database schema.
# It uses the SeaORM CLI to connect to a PostgreSQL database and generate the models.
# Ensure the database is running and accessible
# before executing this script.

# Usage: ./hack/scripts/generate-model.sh

psql --username=postgres --dbname=postgres --host=localhost --port 54322 --file=migrations/001__initial_schema.psql

sea-orm-cli generate entity -u "postgres://postgres:postgres@localhost:54322/postgres" -o src/entity