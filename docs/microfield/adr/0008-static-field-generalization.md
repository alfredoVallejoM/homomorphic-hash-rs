# ADR 0008: generalización estática de campos binarios

- Estado: aceptado
- Fecha: 31 de julio de 2026

## Contexto

H2 implementó `Gf2_256HhV1` directamente sobre cuatro limbs. H3 debe añadir
un campo de 128 bits y otro módulo de 256 bits sin duplicar matemáticas, mezclar
identidades nominales ni introducir indirección en operaciones escalares.

Rust estable no permite expresar de forma general `[u64; 2 * N]` sin depender
de expresiones const genéricas todavía no disponibles en el MSRV. Un wrapper
público genérico también expondría detalles de estrategia en la API.

## Decisión

Cada campo continúa siendo un newtype público nominal sobre un array privado.
El trait privado `BinaryFieldImpl` asocia ese value object con una estrategia
estática. Se proporcionan dos estrategias sin estado:

- `Polynomial128<TAIL>`, con limbs `[u64; 2]` y producto ancho `[u64; 4]`;
- `Polynomial256<TAIL>`, con limbs `[u64; 4]` y producto ancho `[u64; 8]`.

El tail del polinomio es un const generic. Producto carry-less, folds de
reducción, cuadrado, inversión y operaciones de extensión reciben el grado o la
estrategia mediante contratos internos y quedan monomorfizados.

Un macro privado emite el boilerplate de cada newtype: constantes, traits
públicos, operadores, encoding y formato. El macro no contiene el algoritmo de
producto, reducción o inversión.

## Consecuencias

- Los campos de igual cardinal siguen siendo tipos incompatibles.
- Añadir otro tail de 128 o 256 bits no duplica matemáticas.
- No hay `Box<dyn Trait>`, vtable, asignación ni estado de backend en un
  elemento.
- Los anchos soportados son explícitos y compatibles con Rust 1.89.
- Un grado nuevo que requiera otro número de limbs exige una estrategia nueva,
  lo que hace visible su coste y permite auditarla por separado.

## Alternativas rechazadas

- `dyn BinaryFieldImpl`: introduce dispatch y no es apropiado para tipos con
  constantes y representaciones asociadas.
- Un único `BinaryField<const N, const TAIL>` público: filtra representación y
  debilita la identidad nominal.
- Copiar la implementación H2 tres veces: viola responsabilidad única y hace
  posible que los algoritmos diverjan.
- Esperar a `generic_const_exprs`: bloquearía H3 y rompería el MSRV acordado.
