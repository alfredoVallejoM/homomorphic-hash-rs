# Algesum PostgreSQL lab

Laboratorio no publicable para validar la integración de Algesum con una base
transaccional real. Crea exclusivamente `algesum_accounts` y
`algesum_metadata`, carga filas mediante `generate_series`, ejecuta cada lote
en una transacción PostgreSQL y comprueba después de cada commit:

- igualdad exacta entre PostgreSQL y el espejo de filas;
- igualdad del resumen actualizado y una reconstrucción completa;
- traducción de posiciones externas a revisiones contiguas;
- ruta incremental, híbrida, reconstrucción por partición o reconstrucción
  autoritativa elegida por la política.

Las mediciones son de rendimiento, no pruebas de propiedades criptográficas.
Los resúmenes de Algesum son algebraicos y no criptográficos.

## Ejecución reproducible

El comando destruye y recrea solamente las dos tablas anteriores. Debe usarse
una base aislada:

```text
docker run --rm --detach --name algesum-postgres-test \
  -e POSTGRES_PASSWORD=postgres \
  -p 127.0.0.1:55432:5432 postgres:17-alpine

cargo run --release -p algesum-postgres-lab --locked -- \
  --rows 65536 \
  --partitions 256 \
  --repetitions 3 \
  --batches 256,4096,16384,32768,49152,65536 \
  --incremental-ceiling 65536 \
  --partition-density 2/3 \
  --output /tmp/algesum-postgresql-adaptive-v2.json

docker stop algesum-postgres-test
```

`ALGESUM_DATABASE_URL` o `--database-url` permiten seleccionar otra instancia.
Una campaña de publicación deberá usar más repeticiones, una máquina dedicada,
afinidad fija y un directorio de resultados nuevo.
