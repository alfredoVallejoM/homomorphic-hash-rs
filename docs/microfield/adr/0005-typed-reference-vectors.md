# ADR 0005 — Vectores externos tipados

**Estado:** aceptado.

## Contexto

El esquema provisional representaba la operación como texto y aplicaba el
ancho canónico a cualquier entrada o salida. No podía distinguir un producto
polinómico ancho, una entrada de reducción, un exponente ni la ausencia de
inversa de cero.

## Decisión

La certificación externa usa exclusivamente el esquema JSON v2. Cada caso
contiene un nombre estable y una operación enum tipada:

- `canonical`;
- `add`;
- `wide_product`;
- `reduce`;
- `multiply`;
- `square`;
- `invert`;
- `pow`;
- `mul_by_x`.

Para un campo de grado `m` y ancho canónico `b = ceil(m/8)`:

- los elementos ocupan exactamente `b` bytes little-endian;
- los valores polinómicos anchos ocupan exactamente `2b` bytes;
- solo pueden estar activos los primeros `m` bits de un elemento;
- un producto ancho puede usar como máximo `2m-1` bits;
- el exponente usa bytes little-endian mínimos y cero se representa como
  `"00"`;
- la salida de `invert` es `null` si y solo si la entrada es cero.

Cada set registra nombre y versión del oráculo, algoritmo de derivación y seed
de 256 bits. Se exige cobertura de todas las operaciones y de los dominios de
inversión cero/no-cero.

El importador limita el JSON a 8 MiB, 4096 casos y exponentes de 4096 bytes.
Rechaza claves desconocidas, nombres duplicados, mayúsculas, anchos incorrectos
y bits fuera del grado.

## Consecuencias

- El esquema v1 deja de aceptarse.
- El modelo no permite operaciones futuras parcialmente descritas.
- Sage sigue siendo un proceso externo y no entra en el runtime.
- Añadir una operación requiere una nueva variante, validación, cobertura y
  una revisión de esquema.

