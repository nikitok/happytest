# Локально с S3 (Bybit - default)
cargo run --release --bin reader -- \
  --symbol BTCUSDT \
  --interval 600 \
  --parquet \
  --s3-bucket my-trading-data \
  --s3-prefix orderbook/v1

# Binance Futures
cargo run --release --bin reader -- \
  --provider binance \
  --symbol BTCUSDT \
  --interval 600 \
  --parquet \
  --s3-bucket my-trading-data \
  --s3-prefix orderbook/v1

# Docker build
docker build -t happytest/reader:v1.0.0 .

# Helm deploy (Deployment mode - continuous)
helm install btc-reader .helm/happytest-reader \
  --set reader.provider=bybit \
  --set reader.symbol=BTCUSDT \
  --set s3.bucket=my-trading-data

# Helm deploy (CronJob mode - hourly restarts with S3 upload)
helm install btc-reader .helm/happytest-reader \
  --set cronjob.enabled=true \
  --set reader.provider=binance \
  --set reader.symbol=BTCUSDT \
  --set s3.bucket=my-trading-data

# Multiple symbols (deploy multiple releases)
helm install btc-bybit .helm/happytest-reader \
  --set cronjob.enabled=true \
  --set reader.provider=bybit \
  --set reader.symbol=BTCUSDT \
  --set s3.bucket=my-trading-data

helm install eth-binance .helm/happytest-reader \
  --set cronjob.enabled=true \
  --set reader.provider=binance \
  --set reader.symbol=ETHUSDT \
  --set s3.bucket=my-trading-data

Данные будут загружаться в S3 с Athena-совместимой структурой:
s3://bucket/orderbook/v1/symbol=BTCUSDT/date=2024-12-28/file.parquet


Для Athena нужно создать таблицу, которая будет читать Parquet файлы из S3. Вот инструкция:

1. Создание базы данных и таблицы в Athena

Зайди в AWS Console → Athena → Query Editor и выполни:

-- Создать базу данных
CREATE DATABASE IF NOT EXISTS trading;

-- Создать таблицу с партиционированием
CREATE EXTERNAL TABLE IF NOT EXISTS trading.orderbook (
symbol STRING,
timestamp BIGINT,
update_id BIGINT,
fetch_time BIGINT,
bids ARRAY<STRUCT<price:STRING, size:STRING>>,
asks ARRAY<STRUCT<price:STRING, size:STRING>>
)
PARTITIONED BY (
symbol_partition STRING,
date_partition STRING
)
STORED AS PARQUET
LOCATION 's3://YOUR-BUCKET/orderbook/v1/'
TBLPROPERTIES (
'parquet.compression'='SNAPPY',
'projection.enabled'='true',
'projection.symbol_partition.type'='enum',
'projection.symbol_partition.values'='BTCUSDT,ETHUSDT,TONUSDT',
'projection.date_partition.type'='date',
'projection.date_partition.range'='2024-01-01,NOW',
'projection.date_partition.format'='yyyy-MM-dd',
'storage.location.template'='s3://YOUR-BUCKET/orderbook/v1/symbol=${symbol_partition}/date=${date_partition}'
);

Замени YOUR-BUCKET на имя твоего S3 bucket!

2. Примеры запросов

-- Последние 100 записей для BTCUSDT за сегодня
SELECT
symbol,
from_unixtime(timestamp/1000) as time,
bids[1].price as best_bid,
asks[1].price as best_ask
FROM trading.orderbook
WHERE symbol_partition = 'BTCUSDT'
AND date_partition = '2024-12-29'
ORDER BY timestamp DESC
LIMIT 100;

-- Средний спред за день
SELECT
date_partition,
AVG(CAST(asks[1].price AS DOUBLE) - CAST(bids[1].price AS DOUBLE)) as avg_spread
FROM trading.orderbook
WHERE symbol_partition = 'BTCUSDT'
GROUP BY date_partition
ORDER BY date_partition DESC;

-- Количество записей по дням
SELECT
symbol_partition,
date_partition,
COUNT(*) as records
FROM trading.orderbook
GROUP BY symbol_partition, date_partition
ORDER BY date_partition DESC;

3. Без Partition Projection (ручное добавление партиций)

Если не хочешь использовать partition projection, создай таблицу попроще:

CREATE EXTERNAL TABLE IF NOT EXISTS trading.orderbook_simple (
symbol STRING,
timestamp BIGINT,
update_id BIGINT,
fetch_time BIGINT,
bids ARRAY<STRUCT<price:STRING, size:STRING>>,
asks ARRAY<STRUCT<price:STRING, size:STRING>>
)
STORED AS PARQUET
LOCATION 's3://YOUR-BUCKET/orderbook/v1/'
TBLPROPERTIES ('parquet.compression'='SNAPPY');

Но тогда Athena будет сканировать все файлы (дороже!).

4. Настройка результатов Athena

Перед первым запросом укажи S3 bucket для результатов:
1. Athena → Settings → Manage
2. Query result location: s3://YOUR-BUCKET/athena-results/

