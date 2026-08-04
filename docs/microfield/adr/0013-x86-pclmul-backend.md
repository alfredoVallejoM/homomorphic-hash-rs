# ADR 0013 — Backend batch x86-64 PCLMUL

**Estado:** aceptado.

## Contexto

H2.3 dejó separadas compilación, elegibilidad del campo, soporte de CPU y
política. H2.4 debe aprovechar `PCLMULQDQ` sin introducir instrucciones ISA en
el tipo escalar, cambiar el encoding, permitir capabilities falsificables ni
propagar `unsafe` por el dominio.

Los elementos mantenidos tienen alineamiento natural de 8 bytes, no de 16. El
backend tampoco puede exigir packing, scratch o asignación para usar la API AoS
ya publicada.

## Decisión

Se añade una Strategy batch interna `backend::x86_pclmul`, registrada solo en
los catálogos sellados de `Gf2_128V1`, `Gf2_256HhV1` y `Gf2_256AltV1`.
`Field::mul` y `Square::square` continúan siendo portables, estáticos y
monomorfizados; la aceleración se selecciona únicamente al construir
`Engine<F>`.

El producto de 128 bits usa Karatsuba con tres productos carry-less de 64
bits. El de 256 bits aplica un nivel exterior de Karatsuba sobre mitades de 128
bits, con nueve productos carry-less. El cuadrado aprovecha que en
característica dos no existen términos cruzados y usa dos o cuatro productos,
respectivamente. El producto ancho resultante entra en los reductores seguros
y ya certificados `reduce_128`/`reduce_256`; el backend ISA no decide el
polinomio ni el encoding.

Las cargas y stores admiten direcciones con alineamiento de 8 bytes. Los
kernels aceptan cualquier longitud, incluido cero y colas no múltiplo de un
tile, y ofrecen las mismas entradas out-of-place e in-place que portable. Su
metadata declara:

- `minimum_batch = 1`;
- `preferred_multiple = 1`;
- alineamiento natural del elemento;
- sin packing ni scratch;
- in-place soportado;
- schedule fijo respecto de los valores.

## Frontera de seguridad

La raíz del crate y el lint Cargo usan `unsafe_code = "deny"`. Todo `unsafe`
de este backend está en `backend/x86_pclmul.rs`. Tras H2.5 existe una segunda
excepción equivalente en `backend/aarch64_pmull.rs`; un test recorre el árbol y
exige exactamente esos dos sitios.

Los entry points seguros del kernel son privados. Solo pueden alcanzarse desde
una tabla estática que el selector entrega después de validar una
`CpuCapabilities` no falsificable con `pclmulqdq`. `build()` parte de
`portable_only` y no puede ejecutar ISA; `detect()` es la única ruta pública
que habilita la capability real. No se detecta CPU dentro de una operación.

Los campos externos ABI 1/2 mantienen catálogo portable. En x86-64 forzar
PCLMUL sobre ellos devuelve `BackendUnsupportedByField`: que el backend forme
parte del binario no certifica automáticamente una representación externa.
ABI 3 materializa el perfil explícito y versionado exigido por esta decisión.

## Evidencia

- Karatsuba coincide con un schoolbook PCLMUL para casos frontera y 4096
  entradas pseudoaleatorias reproducibles.
- Producto y cuadrado PCLMUL coinciden con la ruta portable independiente para
  los tres presets, cada bit de sus bases y 17 longitudes entre 0 y 16 384.
- Las pruebas cubren cero, uno, bit alto, entradas densas, patrones alternos,
  canarios, in-place y errores transaccionales.
- AddressSanitizer completa la suite pública del backend.
- El contador externo observa cero asignaciones en portable y en el backend
  detectado.
- El desensamblado contiene `pclmullqlqdq`; los kernels batch no contienen
  llamadas indirectas internas ni referencias al asignador.
- Rust 1.89 compila y ejecuta el backend; AArch64 compila ambas configuraciones
  sin incluirlo.

En el Intel Core i7-13700HX medido con Rust 1.97.1, incluso el límite superior
PCLMUL frente al inferior portable mejora el cuadrado de un elemento entre
24,6 % y 35,7 %. La multiplicación mejora mucho más; HH-256/4096 pasa de
1,4687–1,4811 ms a 39,055–39,333 µs. Se fija por ello el umbral automático en
un elemento. Son datos de selección para esta implementación, no una promesa
de latencia universal.

## Consecuencias

- `Auto`, `LowLatency`, `Throughput` y `FixedSchedule` pueden seleccionar
  PCLMUL en una CPU compatible; `PortableOnly` nunca lo hace.
- `expected_batch = 0` conserva portable en `Auto`; forzar PCLMUL sigue siendo
  correcto para un slice vacío porque el umbral no es una precondición.
- El layout, `FieldId`, `ArtifactId`, encoding y API escalar no cambian.
- H2.5 añadió PMULL sin modificar este backend ni `Engine`.

## Alternativas rechazadas

- Compilar todo el crate con `target-cpu=native`: elimina portabilidad del
  binario y no permite fallback seguro.
- Detectar dentro del bucle: añade ramas y debilita la precondición auditable.
- Poner flags ISA en cada elemento: cambia layout y contamina la ruta escalar.
- Exigir 16 bytes o packing para PCLMUL: no aporta ventaja a estas cargas y
  rompe el contrato AoS natural.
- Registrar automáticamente todo campo externo: confunde cardinalidad con una
  representación y un plan de reducción certificados.
