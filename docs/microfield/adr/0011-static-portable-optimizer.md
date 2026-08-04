# ADR 0011 — Optimizador portable estático

**Estado:** aceptado.

## Contexto

H2.1 permitió generar cualquier campo binario v1, pero su ruta general usaba
producto, cuadrado y reducción bit a bit. Era un buen oráculo de corrección,
no una base de rendimiento para la mayoría de consumidores. Crear una
implementación manual por campo no escala y haría divergir certificados,
encoding y mantenimiento.

## Decisión

Un selector puro deriva `PortableOptimizationPlan` solo del descriptor
certificado. La decisión ocurre durante codegen, se serializa en el IR v2 y se
monomorfiza en el módulo generado.

Se seleccionan estas estrategias:

| Problema | Estrategia |
|---|---|
| Producto | schoolbook carry-less por bits activos |
| Cuadrado | expansión directa de bits de cada limb |
| Grado alineado, tail ≤ 32 | fold de palabras acotado en dos etapas |
| Módulo disperso | eliminación descendente por términos no nulos |
| Módulo denso | eliminación descendente con tail empaquetado en palabras |
| Inversión | cadena binaria Itoh–Tsujii |

La clase de grado registra si es potencia de dos alineada, alineada o no
alineada. Sirve para trazabilidad y futuras estrategias, pero la reducción se
elige por alineamiento y módulo; no se presupone que todo grado potencia de dos
sea rápido por sí mismo.

El umbral disperso es determinista: al menos ocho términos de tail o dos por
limb, el mayor. Evita expandir código según el número de términos y limita el
coste de módulos densos.

## Estabilidad y seguridad

- `FieldId`, representación canónica, layout y resultados no cambian.
- El perfil cambia `ArtifactId` y el digest exacto del paquete.
- ABI 2 añade helpers; ABI 1 se conserva durante la ventana N/N-1.
- No hay detección runtime, heap, punteros de estrategia escalar ni trait
  objects.
- Toda la implementación portable sigue bajo `forbid(unsafe_code)`.
- La implementación v1 se conserva como oráculo diferencial interno.
- No se promete tiempo constante: producto e inversión dependen de datos o de
  un schedule que no ha sido auditado como constant-time.

## Consecuencias

- Todos los campos generados reciben producto, cuadrado e inversión mejorados.
- Los módulos comunes dispersos, incluidos grados no alineados como 233,
  evitan el coste denso.
- Grados 64/128/256/512/1024 con tail bajo comparten el fold más barato; el
  helper soporta igualmente 2048/4096 dentro del techo v1.
- Los presets mantenidos pueden conservar implementaciones especializadas sin
  duplicar la política del factory.
- Karatsuba queda pendiente de umbrales medidos; no se activa por intuición.

## Alternativas rechazadas

- Selección runtime por elemento: añade estado y ramas en el hot path.
- Un único algoritmo bit a bit: estable pero insuficiente como ruta de
  producción.
- Desenrollar toda reducción densa: aumenta fuente, tiempo de compilación e
  icache hasta el grado 4096.
- Karatsuba universal: sus cruces dependen de tamaño, compilador y CPU y aún no
  tienen evidencia suficiente.
- Marcar solo potencias de dos como optimizadas: ignora alineamiento y forma
  del polinomio, que son las propiedades relevantes para la reducción.
