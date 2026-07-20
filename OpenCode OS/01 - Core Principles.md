# 01 - Principios Fundamentales

Los principios siguientes son **no negociables**. Cualquier decisión de diseño que los contradiga debe ser rechazada y documentada como excepción.

---

## P1. Motores, no fases

El sistema se divide en **motores independientes**, no en fases secuenciales. Algunos motores funcionan de forma continua (Context, Research, Learning) y otros se disparan bajo demanda (Repair, Validation). Las fases son una abstracción obsoleta del harness previo.

## P2. Restricciones, no consejos

La IA no recibe *"programa bonito"*. Recibe: *"si Biome falla, no avanzas"*. Las reglas escalan mejor que el texto narrativo. El harness empuja **restricciones ejecutables**, no sugerencias.

## P3. Cualquier modelo puede entrar

El sistema no está acoplado a un proveedor. Cualquier modelo —local de 7B en una GPU del usuario, un modelo gratuito vía Nvidia NIM / Google AI / OpenRouter / GitHub / Cerebras / Groq / Sambanova / Cloudflare Workers AI / HuggingFace / Mistral, o un modelo de pago— puede entrar, comprender el estado del proyecto, ejecutar una tarea y salir **sin pérdida de conocimiento**.

## P4. Memoria estructurada antes que prompt

Nunca se depende del prompt para mantener contexto. Toda operación persiste en:
- el **Execution Journal** (estado de ejecución),
- la **Architecture Memory** (mapa del proyecto),
- el **Skill Graph** (capacidades disponibles),
- el **Vector Knowledge Base** (recuperación semántica).

El prompt es solo una lente sobre esa memoria.

## P5. Investigación obligatoria antes de decisiones críticas

Antes de modificar código importante o tomar una bifurcación arquitectónica, el **Research Engine** corre. Sin evidencia, no hay decisión. *"creo que..."* es inaceptable; **se exige *"*el 83% de las fuentes recomienda X*"***.

## P6. El sistema se sabe equivocar

El sistema lleva un **Confidence Score** sobre su propio razonamiento. Si cae bajo el umbral, dispara meta-razonamiento, debate, re-investigación o cambio de modelo. No se emiten respuestas de baja confianza como si fueran definitivas.

## P7. Cada cambio dispara validación incremental

Guardar un archivo no es un evento neutro. Cada guardado dispara el pipeline: Biome → TypeCheck → Tests → E2E → Knip → Snyk → Commit permitido. Nada se consolida sin pasar el filtro.

## P8. El error produce aprendizaje

Cada bug genera **Root Cause → qué regla faltó → nueva regla → guardar harness**. El sistema mejora con el tiempo, no solo el proyecto.

## P9. Seguridad por defecto

El sistema es extensible (MCPs, CLIs, Skills, plugins) pero **sandboxed y auditable**. Las acciones sensibles requieren aprobación. La cadena de suministro se inspecciona (Socket). Esta es la crítica que se le hace a OpenClaw y que aquí se resuelve.

## P10. Doble modo

El editor sigue siendo un editor: el usuario puede escribir código manualmente como siempre. Existen **Modo Manual** y **Modo IA**, y dentro del Modo IA puede restringirse a **modelos locales** o **modelos gratuitos** para ver siempre qué están haciendo.

## P11. Transparencia total

El usuario siempre puede ver qué hace cada agente, qué modelo lo hace, qué evidencia usó y qué regla lo disparó. La orquestación no es una caja negra.

## P12. No se mete todo en un único archivo

El sistema se documenta como un conjunto de RFCs (esta carpeta). Cada motor, protocolo y subsistema tiene su especificación. Esto es arquitectura, no un harness.
