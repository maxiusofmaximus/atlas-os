//! Cross-provider tool-call normalisation.
//!
//! RFC 04 §3 — cascade fallback Anthropic→OpenAI→Gemini rompería si las
//! llamadas a herramientas (`tool_calls` / `tool_use` / `functionCall`)
//! se pasaran como JSON opaco al proveedor equivocado. Cada provider
//! tiene un dialecto distinto para invocar herramientas:
//!
//! - **OpenAI** / **DeepSeek** / **Mistral** / **OpenRouter** (OpenAI-compat):
//!   `tool_calls: [{ id, function: { name, arguments: "<json string>" } }]`
//! - **Anthropic** / **Bedrock (Claude)**: `content: [{
//!   type: "tool_use", id, name, input: <json object> }]`
//! - **Gemini** / **Vertex AI (Gemini)**: `functionCall: { name, args:
//!   <json object> }`
//!
//! El flujo es siempre el mismo: el motor de planificación emite un
//! `OpShape` (forma neutra) que describe la operación pedido; el
//! serializador específico del provider lo transforma a su dialecto
//! al montar el request body, y el deserializador del provider
//! convierte el tool_use devuelto en `OpShape` antes de entregarlo al
//! Coding Engine (RFC 13). Esto desacopla la capa lógica
//! (planificación) de la capa física (wire-format).

use serde::{Deserialize, Serialize};

/// Forma neutra de una llamada a herramienta. Es el único tipo que
/// viaja entre el Coding Engine y el Orchestrator — nadie fuera de
/// `provider::wire` debe tocar `ToolCall` directamente.
///
/// `arguments` se guarda como `serde_json::Value` (objeto) en vez de
/// `String` porque Antrópico y Gemini pasan objects nativos; sólo
/// OpenAI exige serializar el object a `String` en el wire. La
/// conversión a string ocurre en `OpenAI::serialize_tool_call()`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpShape {
    /// Stability id: cada provider genera su propio id (OpenAI usa
    /// `call_abc123`, Anthropic `toolu_xyz`). Aquí guardamos uno
    /// neutro que el Orchestrator asigna — el serializador del
    /// provider lo mapea al prefijo del provider (`call_`/`toolu_`).
    pub id: String,
    pub name: String,
    /// `serde_json::Value::Object` en el caso general. Si el provider
    /// empite un JSON inválido o string, guardamos como `Null` y
    /// notamos el error en logs (no abortamos la cascade por
    /// malformación puntual — el Coding Engine decidirá).
    pub arguments: serde_json::Value,
}

impl OpShape {
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            id: next_id(),
            name: name.into(),
            arguments,
        }
    }

    /// Extrae el `arguments` como texto JSON serializado. Lo necesita
    /// el serializador de OpenAI (`arguments` va como string en el
    /// wire). Devuelve `"null"` si el Value no es objecto — el
    /// Coding Engine verificará.
    pub fn arguments_json(&self) -> String {
        serde_json::to_string(&self.arguments).unwrap_or_else(|_| "null".into())
    }
}

/// Generador de ids neutros. Estilo `opc_` para distinguir rápidente
/// `opencode` de los ids `call_` (OpenAI) y `toolu_` (Anthropic). El
/// serializador del provider reescribe el prefijo.
pub fn next_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("opc_tool_{}", n)
}

/// Dialecto wire-format del tool_call devuelto por el provider.
/// Deserialización durante el parseo de la response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolCall {
    /// OpenAI / DeepSeek / Mistral / OpenRouter-OpenAI-compat.
    /// `arguments` es un string JSON — hay que reparsear a `Value`.
    OpenAI {
        id: String,
        #[serde(default)]
        function: OpenAIToolFunction,
    },
    /// Anthropic / Bedrock Claude. `input` ya es `Value::Object` nativo.
    Anthropic {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    /// Gemini / Vertex Gemini. Args ya son `Value::Object`.
    Gemini {
        name: String,
        #[serde(default)]
        args: serde_json::Value,
    },
    /// Ollama (formato OpenAI-compat) y LlamaCppServer.
    Local {
        id: String,
        name: String,
        #[serde(default)]
        arguments: serde_json::Value,
    },
}

/// Sub-estructura para OpenAI-compat (incluido Ollama cuando hace
/// OpenAI passthrough). Las tools viven bajo `function`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIToolFunction {
    pub name: String,
    /// OpenAI envía `arguments` como **string** de JSON (en el
    /// streaming llega token a token, hay que concatenar antes de
    /// parsear). Aquí dejamos como String; el `normalize()` hace
    /// el parse a `Value`.
    #[serde(default)]
    pub arguments: String,
}

impl ToolCall {
    /// Normaliza cualquier dialecto a `OpShape`. La-id la pasa el
    /// provider; si el dialecto no trae id (Gemini no emite ids
    /// estables hasta `2406.18` release), asignamos `next_id()`.
    pub fn normalize(self) -> Result<OpShape, ToolCallError> {
        match self {
            ToolCall::OpenAI { id, function } => {
                let args: serde_json::Value = if function.arguments.is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    serde_json::from_str(&function.arguments).unwrap_or(serde_json::Value::Null)
                };
                Ok(OpShape {
                    id,
                    name: function.name,
                    arguments: args,
                })
            }
            ToolCall::Anthropic { id, name, input } => {
                let args = if input.is_null() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    input
                };
                Ok(OpShape {
                    id,
                    name,
                    arguments: args,
                })
            }
            ToolCall::Gemini { name, args } => {
                let id = next_id();
                let a = if args.is_null() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    args
                };
                Ok(OpShape {
                    id,
                    name,
                    arguments: a,
                })
            }
            ToolCall::Local {
                id,
                name,
                arguments,
            } => {
                let a = if arguments.is_null() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    arguments
                };
                Ok(OpShape {
                    id,
                    name,
                    arguments: a,
                })
            }
        }
    }

    /// Convierte un `OpShape` neutro al dialecto específico para
    /// enviarlo **dentro** del request body al provider objetivo
    /// (escritura de tools, no lectura — se invoca al montar el
    /// `tools: [...]` array de los parámetros que el provider
    /// soporta).
    pub fn denormalize(shape: &OpShape, target: ProviderWire) -> ToolCall {
        match target {
            ProviderWire::OpenAI
            | ProviderWire::DeepSeek
            | ProviderWire::Mistral
            | ProviderWire::Groq
            | ProviderWire::Cerebras
            | ProviderWire::Sambanova
            | ProviderWire::OpenRouter
            | ProviderWire::LiteLLM
            | ProviderWire::GitHubModels
            | ProviderWire::NvidiaNim
            | ProviderWire::HuggingFaceInference => ToolCall::OpenAI {
                id: prefix_id(&shape.id, "call"),
                function: OpenAIToolFunction {
                    name: shape.name.clone(),
                    arguments: shape.arguments_json(),
                },
            },
            ProviderWire::Anthropic | ProviderWire::Bedrock => ToolCall::Anthropic {
                id: prefix_id(&shape.id, "toolu"),
                name: shape.name.clone(),
                input: shape.arguments.clone(),
            },
            ProviderWire::Gemini | ProviderWire::VertexAI => ToolCall::Gemini {
                name: shape.name.clone(),
                args: shape.arguments.clone(),
            },
            ProviderWire::Ollama
            | ProviderWire::LmStudio
            | ProviderWire::LlamaCppServer
            | ProviderWire::CloudflareWorkersAi => ToolCall::Local {
                id: prefix_id(&shape.id, "call"),
                name: shape.name.clone(),
                arguments: shape.arguments.clone(),
            },
            ProviderWire::Azure => ToolCall::OpenAI {
                id: prefix_id(&shape.id, "call"),
                function: OpenAIToolFunction {
                    name: shape.name.clone(),
                    arguments: shape.arguments_json(),
                },
            },
            ProviderWire::Custom(_) => ToolCall::OpenAI {
                id: prefix_id(&shape.id, "call"),
                function: OpenAIToolFunction {
                    name: shape.name.clone(),
                    arguments: shape.arguments_json(),
                },
            },
        }
    }
}

/// Reescribe el prefijo del id (`opc_tool_42` → `call_opc_tool_42`
/// para OpenAI, `toolu_opc_tool_42` para Anthropic). El id
/// subyacente del `OpShape` se mantiene; sólo se cambia el prefijo
/// en el wire.
fn prefix_id(id: &str, prefix: &str) -> String {
    if id.starts_with(prefix) {
        id.to_string()
    } else {
        format!("{}_{}", prefix, id)
    }
}

/// Error de normalización. Hoy en día sólo registra malformación;
/// el Orchestrator decide si aborta o sigue cascade. Ver RFC 13.
#[derive(Debug, thiserror::Error)]
pub enum ToolCallError {
    #[error("invalid arguments JSON: {0}")]
    InvalidArgumentsJson(#[from] serde_json::Error),
}

use crate::orchestrator::provider::ProviderWire;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn op_shape_arguments_json_serialises_object_to_string() {
        let s = OpShape::new("calc", json!({"a": 1, "b": 2}));
        assert_eq!(s.arguments_json(), r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn op_shape_arguments_json_null_when_invalid_value() {
        let s = OpShape::new("noop", json!(null));
        assert_eq!(s.arguments_json(), "null");
    }

    #[test]
    fn op_shape_next_id_is_monotonic_unique() {
        let a = next_id();
        let b = next_id();
        assert_ne!(a, b, "ids monotonicos deben diferir");
        assert!(a.starts_with("opc_tool_"));
    }

    #[test]
    fn tool_call_openai_normalize_parses_arguments_string_to_value() {
        let tc = ToolCall::OpenAI {
            id: "call_1".into(),
            function: OpenAIToolFunction {
                name: "ls".into(),
                arguments: r#"{"path":"."}"#.into(),
            },
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.id, "call_1");
        assert_eq!(shape.name, "ls");
        assert_eq!(shape.arguments, json!({"path": "."}));
    }

    #[test]
    fn tool_call_openai_normalize_handles_empty_arguments_as_empty_object() {
        let tc = ToolCall::OpenAI {
            id: "call_2".into(),
            function: OpenAIToolFunction {
                name: "noop".into(),
                arguments: String::new(),
            },
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.arguments, json!({}));
    }

    #[test]
    fn tool_call_anthropic_normalize_uses_input_object_directly() {
        let tc = ToolCall::Anthropic {
            id: "toolu_a".into(),
            name: "write".into(),
            input: json!({"file": "x", "content": "y"}),
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.id, "toolu_a");
        assert_eq!(shape.name, "write");
        assert_eq!(shape.arguments, json!({"file":"x","content":"y"}));
    }

    #[test]
    fn tool_call_anthropic_normalize_null_input_becomes_empty_object() {
        let tc = ToolCall::Anthropic {
            id: "toolu_a".into(),
            name: "noop".into(),
            input: json!(null),
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.arguments, json!({}));
    }

    #[test]
    fn tool_call_gemini_normalize_assigns_fresh_id() {
        let tc = ToolCall::Gemini {
            name: "search".into(),
            args: json!({"q": "rust"}),
        };
        let shape = tc.normalize().unwrap();
        assert!(shape.id.starts_with("opc_tool_"), "Gemini sin id → fresh");
        assert_eq!(shape.name, "search");
        assert_eq!(shape.arguments, json!({"q": "rust"}));
    }

    #[test]
    fn tool_call_gemini_normalize_null_args_becomes_empty_object() {
        let tc = ToolCall::Gemini {
            name: "noop".into(),
            args: json!(null),
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.arguments, json!({}));
    }

    #[test]
    fn tool_call_local_treats_arguments_as_value_object() {
        let tc = ToolCall::Local {
            id: "call_local".into(),
            name: "exec".into(),
            arguments: json!({"cmd":"echo hi"}),
        };
        let shape = tc.normalize().unwrap();
        assert_eq!(shape.arguments, json!({"cmd":"echo hi"}));
    }

    #[test]
    fn tool_call_roundtrip_anthropic_to_openai_via_op_shape() {
        let original = ToolCall::Anthropic {
            id: "toolu_x".into(),
            name: "calc".into(),
            input: json!({"a": 1}),
        };
        let shape = original.normalize().unwrap();
        // serializo a OpenAI dialect
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::OpenAI);
        match re_emit {
            ToolCall::OpenAI { id, function } => {
                assert!(id.starts_with("call_"), "OpenAI prefix");
                assert_eq!(function.name, "calc");
                let parsed: serde_json::Value = serde_json::from_str(&function.arguments).unwrap();
                assert_eq!(parsed, json!({"a": 1}));
            }
            other => panic!("esperaba OpenAI, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_roundtrip_openai_to_anthropic_via_op_shape() {
        let original = ToolCall::OpenAI {
            id: "call_a".into(),
            function: OpenAIToolFunction {
                name: "cat".into(),
                arguments: r#"{"path":"/etc/hosts"}"#.into(),
            },
        };
        let shape = original.normalize().unwrap();
        // serializo a Anthropic dialect
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Anthropic);
        match re_emit {
            ToolCall::Anthropic { id, name, input } => {
                assert!(id.starts_with("toolu_"), "Anthropic prefix");
                assert_eq!(name, "cat");
                assert_eq!(input, json!({"path":"/etc/hosts"}));
            }
            other => panic!("esperaba Anthropic, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_roundtrip_gemini_to_openai_via_op_shape() {
        let original = ToolCall::Gemini {
            name: "search".into(),
            args: json!({"q": "rust"}),
        };
        let shape = original.normalize().unwrap();
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::DeepSeek);
        match re_emit {
            ToolCall::OpenAI { function, .. } => {
                assert_eq!(function.name, "search");
                let parsed: serde_json::Value = serde_json::from_str(&function.arguments).unwrap();
                assert_eq!(parsed, json!({"q":"rust"}));
            }
            other => panic!("esperaba OpenAI, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_denormalize_anthropic_keeps_input_as_native_value() {
        let shape = OpShape::new("write", json!({"path": "/tmp/f"}));
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Anthropic);
        match re_emit {
            ToolCall::Anthropic { name, input, .. } => {
                assert_eq!(name, "write");
                // No se convierte a string: Anthropic acepta object
                // nativo en `input`.
                assert_eq!(input, json!({"path":"/tmp/f"}));
            }
            other => panic!("esperaba Anthropic, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_denormalize_gemini_keeps_args_object() {
        let shape = OpShape::new("lookup", json!({"key": "id_42"}));
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Gemini);
        match re_emit {
            ToolCall::Gemini { name, args } => {
                assert_eq!(name, "lookup");
                assert_eq!(args, json!({"key":"id_42"}));
            }
            other => panic!("esperaba Gemini, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_denormalize_local_uses_value_object() {
        let shape = OpShape::new("exec", json!({"ps": true}));
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Ollama);
        match re_emit {
            ToolCall::Local {
                id,
                name,
                arguments,
            } => {
                assert!(id.starts_with("call_"), "Local usa prefijo call_");
                assert_eq!(name, "exec");
                assert_eq!(arguments, json!({"ps": true}));
            }
            other => panic!("esperaba Local, obtuve {other:?}"),
        }
    }

    #[test]
    fn tool_call_denormalize_azure_uses_openai_dialect() {
        let shape = OpShape::new("kill", json!({"pid": 1}));
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Azure);
        assert!(matches!(re_emit, ToolCall::OpenAI { .. }));
    }

    #[test]
    fn tool_call_denormalize_custom_uses_openai_dialect_fallback() {
        let shape = OpShape::new("custom_op", json!({"x":1}));
        let re_emit = ToolCall::denormalize(&shape, ProviderWire::Custom("vllm".into()));
        assert!(matches!(re_emit, ToolCall::OpenAI { .. }));
    }

    #[test]
    fn op_shape_serde_roundtrips() {
        let s = OpShape::new("foo", json!({"bar": 42}));
        let js = serde_json::to_string(&s).unwrap();
        let parsed: OpShape = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn tool_call_serde_anthropic_roundtrips() {
        let tc = ToolCall::Anthropic {
            id: "toolu_y".into(),
            name: "edit".into(),
            input: json!({"file":"a.rs","content":"fn x(){}"}),
        };
        let js = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed, tc);
    }

    #[test]
    fn tool_call_serde_openai_roundtrips() {
        let tc = ToolCall::OpenAI {
            id: "call_z".into(),
            function: OpenAIToolFunction {
                name: "ls".into(),
                arguments: r#"{"path":"/tmp"}"#.into(),
            },
        };
        let js = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed, tc);
    }

    #[test]
    fn tool_call_serde_gemini_roundtrips() {
        let tc = ToolCall::Gemini {
            name: "calc".into(),
            args: json!({"a": 1}),
        };
        let js = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed, tc);
    }

    #[test]
    fn prefix_id_preserves_existing_prefix() {
        assert_eq!(prefix_id("call_x", "call"), "call_x");
        assert_eq!(prefix_id("toolu_y", "toolu"), "toolu_y");
    }

    #[test]
    fn prefix_id_adds_prefix_when_missing() {
        assert_eq!(prefix_id("opc_tool_1", "call"), "call_opc_tool_1");
        assert_eq!(prefix_id("opc_tool_2", "toolu"), "toolu_opc_tool_2");
    }
}
