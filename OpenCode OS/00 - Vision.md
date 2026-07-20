# 00 - Visión

> **OpenCode OS** — Agent Engineering Operating System (AEOS) Specification v1

## 1. La tesis

No construir un editor que genera código, sino un **sistema operativo que coordina múltiples modelos, investiga como un arquitecto de software, razona como un ingeniero senior, valida como un pipeline de CI/CD y aprende de cada error para mejorar continuamente**.

El editor deja de ser una colección de herramientas y se convierte en una **plataforma para agentes**.

## 2. El problema que resuelve

Los editores con IA actuales (Cursor, Windsurf, Claude Code, Hermes, OpenClaw) comparten tres debilidades estructurales:

1. **Giran alrededor del prompt.** El harness sigue siendo "instrucciones para que la IA programe bonito". No hay máquina de decisión.
2. **Son mono-modelo o acoplados a un proveedor.** No saben *cuándo* cambiar de cerebro.
3. **Pierden contexto al pasar de un archivo a otro.** Modifican un módulo y olvidan el resto del proyecto.

OpenCode OS no resuelve esto con un mejor prompt. Lo resuelve con **arquitectura**.

## 3. La diferencia fundamental

| Editor tradicional | OpenCode OS |
|---|---|
| Colección de skills | Sistema de motores coordinados |
| Un modelo piensa | Un orquestador decide qué modelo piensa |
| La IA recibe consejos | La IA recibe restricciones |
| El contexto se pierde | Memoria arquitectónica persistente |
| El prompt lo decide todo | El razonamiento se verifica |
| El error lo corrige el usuario | El sistema se repara y aprende |
| El TODO.md es memoria externa | Un Journal estructurado es memoria interna |
| Las skills se acumulan | Las skills se comprimen y evolucionan |

## 4. La filosofía de diseño

El sistema no se pregunta *"¿qué herramienta uso?"*.

Se pregunta *"¿cómo piensa, investiga, decide, programa, valida y aprende un ingeniero senior?"*.

Y luego repo *cada una de esas capacidades* como un motor independiente que puede ser ejercido por **cualquier modelo**, desde un local de 7B hasta un modelo de frontera.

## 5. El mayor diferencial

> Todo el sistema está construido para que cualquier modelo, desde uno local de 7B hasta un modelo de frontera, pueda entrar, comprender el estado exacto del proyecto en pocos segundos, colaborar con otros agentes y abandonar la ejecución sin que se pierda conocimiento.

Eso convierte al editor en una **plataforma para agentes**, no en un cliente de modelos.

Esa es la visión. El resto de esta especificación define cómo se implementa.

## 6. Nombre interno

- **Nombre del producto:** OpenCode OS
- **Nombre técnico:** AEOS (Agent Engineering Operating System)
- **Versión:** v1.0
- **Naturaleza:** Sistema operativo para agentes de programación
- **Licencia:** Abierta y gobernable (ver `18 - Security.md` y `01 - Core Principles.md`)

## 7. Frente a la competencia

- **No competimos** copiando a Hermes ni a OpenClaw.
- **Competimos** haciendo el sistema distribuido, verificable, seguro y auto-evolutivo.
- **No es mejor porque tenga un mejor prompt.** Es mejor porque sabe *cuándo cambiar de cerebro*, sabe *cuándo está pensando mal* y sabe * desde dónde reanudar sin perder conocimiento*.
