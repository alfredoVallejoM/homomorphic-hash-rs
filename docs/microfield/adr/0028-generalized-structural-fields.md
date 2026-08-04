# ADR 0028 — Firmas generalizadas sobre campos estáticos y dinámicos

- Estado: aceptado
- Fecha: 2 de agosto de 2026

## Contexto

Las primeras firmas F6 se escribieron como genéricos sobre `F`, pero era
necesario probar que esa generalidad no dependía accidentalmente de los seis
tipos mantenidos. También se necesitaba consumir campos construidos en runtime
sin introducir checks, heap o dispatch dinámico en la ruta estática. Por último,
una sola evaluación Horner o de producto pierde información que puede ser útil
durante el futuro refinamiento de grafos.

## Decisión

1. Los tipos estáticos siguen siendo la ruta de rendimiento y dependen de
   traits pequeños de `microfield`.
2. `CanonicalElementEncoder` ofrece ingestión directa de `F`; no hay round-trip
   por bytes cuando el llamador ya posee un elemento válido.
3. Un crate de integración genera GF(2⁹) desde TOML durante el build y ejecuta
   todas las firmas. Esto prueba la extensión por factory, no solo por presets.
4. La feature `dynamic-fields` publica tipos distintos que poseen `DynField` y
   `DynElement`. No se añade un enum dinámico dentro de las firmas estáticas.
5. Estático y dinámico comparten `FieldId`, `EncoderId`, `SignatureLaw`,
   `SignatureId` y wire `MFSG`; una misma definición y parámetros producen los
   mismos bytes.
6. Se añade una evaluación de secuencia en ambas orientaciones. Su composición
   conserva las leyes exactas de concatenación.
7. Se añade producto de multiconjunto en `K` offsets pairwise-distinct. Cada
   coordenada conserva su propio contador de factores cero.
8. El número, orden y bytes canónicos de base/offsets forman parte de la
   identidad. Ningún estado con parámetros distintos puede combinarse.

## Consecuencias

Positivas:

- los campos binarios externos generados reciben optimización estática sin
  modificar `structural`;
- configuración runtime y ejecución estática quedan conectadas por una
  representación canónica verificable;
- la segunda orientación y las evaluaciones adicionales conservan más
  estructura sin almacenar la colección completa;
- la ruta estática continúa sin asignaciones por actualización inline.

Costes:

- `DynField` usa ownership, validaciones y potenciales asignaciones; no es un
  sustituto de rendimiento para codegen;
- una secuencia bidireccional realiza aproximadamente el doble de productos;
- `K` evaluaciones cuestan `K` productos y `K` elementos de estado;
- ninguna cantidad finita de evaluaciones elimina colisiones.

## Alternativas rechazadas

- **Registrar campos mediante `Box<dyn Trait>`:** introduce dispatch y rompe la
  selección estática de kernels en la ruta frecuente.
- **Hardcodear cada campo en las firmas:** duplica lógica y contradice la
  factory ya validada.
- **Usar solo potencias de Frobenius como evaluaciones extra:** en campos
  finitos son transformaciones algebraicamente relacionadas y no aportan la
  independencia semántica que sugiere el nombre.
- **Prometer igualdad con suficientes puntos:** el grado/multiplicidad no está
  acotado de forma que convierta el fingerprint en prueba exacta.
