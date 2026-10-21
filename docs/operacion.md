# Operación y cierre

Un cierre reproducible requiere fijar la versión del motor, el periodo, el
orden de inputs y la procedencia de receipts y ajustes. Sable calcula un
`CloseBatch` y un digest, pero el operador sigue siendo responsable de la
aprobación y de la conservación del expediente.

## Rutina de control

1. Comprobar que las referencias externas de factura son únicas en la fuente.
2. Verificar importes y canales de cada receipt contra su archivo original.
3. Confirmar aprobaciones de ajustes y límites de basis points.
4. Ejecutar la preparación del lote y verificar que ningún error dejó
   publicación parcial.
5. Conciliar `total_debits` y `total_credits`, y revisar filas de invoice y
   claim individualmente antes del cierre.
6. Archivar JSON, digest, versión y evidencias de aprobación.

## Alertas recomendadas

- Trial balance no balanceado o digest distinto ante replay idéntico.
- Incremento repentino de claims congelados o cancelados.
- Receipts sin matching, referencias reutilizadas o días fuera de periodo.
- Ajustes próximos al límite individual o acumulado de política.
- Diferencias de totales entre reportes sucesivos sin transacciones nuevas.

## Recuperación

Ante una divergencia, detener la admisión, preservar inputs y salida original,
reproducir el periodo en una instancia aislada y comparar el digest y las
filas relevantes. No corregir saldos mediante edición directa del JSON. El
servicio externo debe disponer de transacciones duraderas para el par
`SettlementBook`/`Ledger`, porque la atomicidad del motor solo cubre memoria.

Un release identifica un commit compartido por `main`, `production` y el tag
anotado; la verificación de integridad de GitHub comprueba también banner y
documentación. Esta versión no implica conexión con infraestructura bancaria.
