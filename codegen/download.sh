#!/bin/bash
curl -fsSL https://raw.githubusercontent.com/postgres/postgres/refs/heads/master/src/backend/utils/errcodes.txt -o ./codegen/errcodes.txt &
curl -fsSL https://raw.githubusercontent.com/postgres/postgres/refs/heads/master/src/include/catalog/pg_type.dat -o ./codegen/pg_type.dat &
curl -fsSL https://raw.githubusercontent.com/postgres/postgres/refs/heads/master/src/include/catalog/pg_range.dat -o ./codegen/pg_range.dat &

wait $(jobs -p)
