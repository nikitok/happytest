#!/bin/bash

# Скрипт для кроссплатформенной сборки Rust проекта под Linux x86_64

set -e

echo "Создание образа для сборки..."
docker build -f Dockerfile.build -t happytest-builder .

echo "Создание директории для результата..."
mkdir -p ./target/linux-release

echo "Запуск контейнера для сборки..."
docker run --rm \
    -v "$(pwd)/target/linux-release:/output" \
    happytest-builder \
    sh -c "cp /app/target/x86_64-unknown-linux-gnu/release/happytest /output/ && cp /app/target/x86_64-unknown-linux-gnu/release/reader /output/"

echo "Сборка завершена! Исполняемые файлы находятся в ./target/linux-release/"
ls -la ./target/linux-release/
