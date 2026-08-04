# ADR 0030 — Discriminación global v2 y corpus externo reproducible

- Estado: aceptado
- Fecha: 2 de agosto de 2026
- Sustituye como ruta recomendada al uso aislado de la firma local v1

## Contexto

F6.G5 demostró correctamente que una recurrencia local no separa `C6` de
`C3 ⊔ C3`, pero la arquitectura seguía ofreciendo esa recurrencia como evidencia
principal. Detectar la colisión y recurrir inmediatamente a una búsqueda
exponencial no satisfacía el objetivo práctico de clasificar grafos grandes con
muy baja latencia.

El fallo no era de F251, GF(2^256) ni SHA-256. Los dos grafos entregaban el
mismo descriptor local exacto. Faltaba información global barata y una política
de escalado entre la firma local y la canonización general.

## Decisión

1. `FastGraphSignature` conserva schema v1 para composición y compatibilidad.
2. `analyze_discriminating` es la fachada recomendada v2. Combina la firma
   algebraica, el canal SHA local, un descriptor global exacto y motivos
   acotados.
3. `GlobalGraphProfile` calcula componentes débiles, SCC, tamaños, tipos,
   etiquetas, relaciones, roles, grados, multiplicidades, bucles, soporte y
   rango cíclico. Su igualdad compara bytes completos; SHA-256 es solo una
   identidad cómoda.
4. Etiquetas y relaciones se internan una vez y los registros ordenan IDs
   canónicos. Esto evita copiar bytes por vértice/arista y conserva exactitud.
5. En alta regularidad, la política `Adaptive` cuenta triángulos y `K4` sobre
   el soporte simple solo si una cota de trabajo invariante cabe en presupuesto.
   Si no cabe, publica `SkippedBudget` sin conteos parciales.
6. Una diferencia en cualquier canal demuestra no isomorfismo bajo el modelo;
   igualdad sigue devolviendo `Indistinguishable`.
7. La canonización exacta descompone componentes débiles, canoniza cada uno con
   un presupuesto global restante, ordena sus formas exactas y solo entonces
   publica el representante total.
8. Los datos externos no entran en Git ni en el test ordinario. Un manifiesto
   fija URL, SHA-256, procedencia y licencia; un fetcher stdlib los deposita en
   caché y los tests quedan marcados `ignored`.

## Consecuencias

`C6` y `C3 ⊔ C3` se separan en el nivel global. Shrikhande y torres 4x4 tienen
el mismo descriptor global de primer orden y los mismos 32 triángulos, pero
`K4=0` frente a `K4=8` los separa dentro del nivel acotado.

El Graph Atlas completo aporta 1.253 representantes no isomorfos hasta siete
vértices: el perfil v2 no deja pares indistinguibles en ese corpus y conserva
el resultado al invertir la numeración. Esto es evidencia empírica fuerte, no
una demostración universal de inyectividad.

La ruta global pura medida sobre un ciclo de 131.072 vértices tarda alrededor
de 23 ms en el host de desarrollo. El discriminador completo sigue siendo un
producto acotado, pero asigna memoria para su resultado owned y no sustituye a
`PreparedGraph`/`GraphWorkspace` cuando solo se necesita throughput local.

## Alternativas rechazadas

- aumentar rondas o lanes: no añade información global ausente;
- hacer SHA-256 del grafo en orden de entrada: rompe invariancia;
- ejecutar canonización exacta por defecto: pierde predictibilidad;
- publicar conteos de motivos parciales: el resultado dependería del orden de
  recorrido;
- descargar datos durante cada `cargo test`: introduce red, disponibilidad y
  cambios upstream en el gate ordinario;
- sustituir v1: rompería composición, identidades y artefactos existentes.
