# ADR 0007 — Vertical portable GF(2²⁵⁶)

**Estado:** aceptado.

## Contexto

H2 necesita un primer campo grande completo sin fijar decisiones ISA ni
introducir representación pública. Ejecutar literalmente los 255 pasos
bit-level del plan de reducción sería verificable, pero degradaría el hot path.

## Decisión

`Gf2_256HhV1` es un newtype `#[repr(transparent)]` sobre cuatro limbs privados.
El módulo `binary` separa cinco responsabilidades:

- `clmul64` y producto escolar ancho;
- reducción word-level;
- expansión dedicada de bits para cuadrado;
- cadena fija de inversión;
- reducción streaming de polinomios arbitrariamente largos.

La reducción usa un const generic con el tail derivado de los exponentes
generados. Para tails de grado máximo 32, un primer fold puede desbordar solo
un limb y un segundo fold termina la reducción. La cota queda comprobada en el
algoritmo y ambos módulos de grado 256 la satisfacen.

La inversión sigue exactamente la cadena `binary-fixed-chain-v1` para
`2^256-2`. Frobenius y traza reutilizan el cuadrado especializado. La norma a
GF(2) usa el hecho de que todo elemento no nulo tiene norma uno.

## Evidencia

- 128 productos deterministas contra división polinómica bit a bit;
- leyes de campo y extensión;
- las 11 operaciones SageMath mantenidas;
- compatibilidad byte a byte con `GaloisSignature256`;
- Miri con el campo grande habilitado;
- release `no_std` sin símbolos de asignador;
- aritmética escalar sin indirect calls ni `Engine`.

## Consecuencias

- El elemento mide 32 bytes y se alinea a 8, no a 32.
- No existe promesa de tiempo constante.
- La optimización ISA seguirá siendo un adaptador interno posterior.
- H3 puede reutilizar el producto y el fold de 256 bits con otro tail, y deberá
  añadir la variante de dos limbs para GF(2¹²⁸).
