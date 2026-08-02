# ADR 0027 — Firmas estructurales, no pruebas criptográficas

- Estado: aceptado
- Fecha: 2 de agosto de 2026

## Contexto

El legado llamaba “proof” a un residuo obtenido aplicando la inversa de una
agregación. Para el producto en un campo, todo factor no nulo es invertible;
para suma y Horner puede despejarse igualmente un término supuesto. La ecuación
recompone el estado, pero no demuestra que el término perteneciera a la
colección histórica.

El proyecto busca hashes homomórficos no criptográficos: interesa conservar
leyes algebraicas y propiedades baratas, no resistencia adversarial.

## Decisión

1. El término público nuevo es `AlgebraicResidual`.
2. Un residual verifica únicamente una ecuación bajo un `SignatureId`.
3. Pertenencia, multiplicidad u orden exactos requieren estado rastreado o un
   certificado estructural futuro independiente.
4. Toda firma contiene o deriva `FieldId`, `EncoderId`, ley y parámetros.
5. SHA-256 compacta estas identidades, sin convertir la firma en criptográfica.
6. La suma, Horner y producto se exponen como tipos distintos; no existe un
   agregador universal con semántica implícita.
7. Los nombres `ProofGenerator` y `ProofVerifier` sobreviven solo para
   compatibilidad fuente y su documentación niega la interpretación antigua.
8. Ninguna colisión de campo podrá decidir isomorfismo en el canonizador.

## Consecuencias

Positivas:

- las garantías coinciden con las matemáticas realmente ejecutadas;
- se conservan composición y compactación sin falsa seguridad;
- los errores de identidad y metadatos fallan cerrados;
- las aplicaciones pueden elegir explícitamente entre estado compacto y
  tracking exacto.

Costes:

- una firma sola no ofrece delete/membership verificado;
- el tracking exacto usa memoria proporcional a la estructura;
- consumidores del vocabulario antiguo deben migrar de “proof” a “residual”;
- la canonización necesita un algoritmo exacto independiente de las firmas.

## Alternativas rechazadas

- **Mantener el nombre proof con una advertencia:** perpetúa una garantía falsa.
- **Añadir sal/hash secreto:** cambia el objetivo a criptografía y no arregla
  la tautología algebraica.
- **Prohibir todo residuo:** elimina una operación útil para rollback bajo una
  hipótesis explícita.
- **Usar el producto como membership filter:** cualquier no cero divide; no
  existe información suficiente.
- **Guardar toda la colección dentro de la firma:** deja de ser compacta; esa
  responsabilidad pertenece a `Tracked*`.
