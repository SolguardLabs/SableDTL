# Sable DTL

![banner](./assets/banner.png)

Sable DTL es un binario Rust para conciliacion de pagos DTL con facturas,
receipts bancarios, ajustes manuales, claims de settlement y cierre contable por
lote. El proyecto modela un motor operativo determinista pensado para auditoria:
ledger balanceado, estados de factura, reportes de aging, politicas de batch y
escenarios reproducibles desde CLI.

El motor usa centavos enteros y un diario de partida doble. Las salidas JSON
están pensadas para reconciliación, observabilidad e integración con un
secuenciador externo; los escenarios incluidos son ejemplos deterministas del
ciclo operativo.

## Circuito económico

```mermaid
flowchart LR
    I[Factura emitida] --> R[Receipts importados]
    I --> A[Ajustes aprobados]
    R --> L[Diario contable]
    A --> L
    I --> S[Obligaciones de settlement]
    S --> C[Claims por lote]
    C --> L
    L --> B[Trial balance]
    B --> P[Cierre del periodo]
    S --> P
```

Ejemplo: una factura de 20.000,00 y un receipt de 7.500,00 mantienen una
obligación pendiente de 12.500,00. Si se registra un ajuste comercial
aprobado, el asiento correspondiente debe mantener iguales débitos y créditos.
Los importes de los reports se expresan en centavos; `20.000,00` se serializa
como `2_000_000`.

## Estados de la operación

```mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> Issued: emisión
    Issued --> PartiallyCollected: receipt o ajuste
    PartiallyCollected --> Paid: saldo contable cero
    Issued --> Paid: pago íntegro
    Paid --> Closed: cierre elegible
    Draft --> Voided: anulación
    Issued --> Voided: anulación sin receipts
```

Un claim tiene un ciclo separado: `Prepared → Posted → Settled`, con ramas
operativas `Frozen` y `Cancelled`. Publicar un lote de claims es atómico dentro
del estado en memoria: si una entrada falla, ni los claims anteriores ni el
diario quedan parcialmente publicados.

## Componentes Principales

- Facturas con terminos de pago, line items, impuestos y estados de cobro.
- Receipts importados desde ACH, wire, card network, rail digital e interno.
- Ajustes manuales aprobados con politica de basis points y doble aprobacion.
- Ledger con chart of accounts, journal balanceado y trial balance.
- Settlement book con obligaciones, claims, freeze/cancel/settle y clearing.
- Cierre mensual con digest canonico, aging, exposicion y evidencia operativa.
- Catalogo de controles para operaciones de conciliacion y cierre.
- Validación de referencias, líneas positivas y fechas de vencimiento sin
  overflow al emitir facturas.
- Publicación atómica de lotes de claims, incluyendo el diario asociado.

## Documentación

- [Arquitectura](docs/arquitectura.md): módulos, contratos y fronteras de
  atomicidad.
- [Modelo económico](docs/modelo-economico.md): centavos, ajustes, diario y
  ejemplos de conciliación.
- [Integración](docs/integracion.md): CLI, estructura JSON y manejo de errores.
- [Operación](docs/operacion.md): controles de cierre y observabilidad.
- [Seguridad operativa](docs/seguridad-operativa.md): límites de confianza y
  respuesta a incidencias.

## Requisitos

- Rust 1.96 o superior.
- Node.js 20 o superior.

## Uso

Ejecutar el escenario final:

```bash
cargo run --quiet
```

Ejecutar escenarios concretos:

```bash
cargo run --quiet -- valid
cargo run --quiet -- receipts
cargo run --quiet -- adjustments
cargo run --quiet -- monthly-close
cargo run --quiet -- final
```

Cada escenario devuelve un reporte JSON con metricas de facturas, ledger,
settlement, close batch, filas de invoice, filas de claim, eventos y digest
determinista.

## Pruebas

Suite Rust:

```bash
cargo test --locked
```

Suite JavaScript:

```bash
node --test tests/node/*.test.js
```

Suite completa:

```bash
bash scripts/tests.sh
```

Validacion de CI local:

```bash
bash scripts/ci.sh
```

## Estructura

```text
src/
  adjustments/  Ajustes manuales, politica y aplicacion contable.
  amount/       Unidades monetarias, bps, ratios y allocations.
  analytics/    Aging, exposure y statements por contraparte.
  close/        Periodos, close batches y reportes de reconciliacion.
  codec/        JSON canonico y digests Blake3.
  error/        Tipos de error del dominio.
  fixtures/     Estado semilla, catalogos y controles operativos.
  ids/          Tipos fuertes para invoices, receipts, batches y accounts.
  invoice/      Facturas, line items, terminos y libro de invoices.
  ledger/       Chart of accounts, journal y trial balance.
  party/        Contrapartes, roles y cuentas bancarias.
  policy/       Politicas de batch y close.
  receipts/     Receipts de settlement y registro de importacion.
  runtime/      CLI y escenarios reproducibles.
  settlement/   Obligaciones, claims y settlement book.
tests/
  node/         Pruebas de integracion sobre la CLI.
  helpers/      Utilidades de ejecucion de escenarios.
```

## CI

El workflow de GitHub Actions ejecuta:

```bash
cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
node --test tests/node/*.test.js
```

Dependabot cubre Cargo, npm y GitHub Actions.

## Alcance de la distribución

El binario ejecuta escenarios locales y no conecta con bancos ni redes de
pago. Autenticación de operadores, conectores externos, persistencia duradera y
control de despliegue corresponden al servicio que lo integre. La etiqueta de
release identifica una versión reproducible del motor; no certifica un
despliegue conectado a fondos de terceros.
