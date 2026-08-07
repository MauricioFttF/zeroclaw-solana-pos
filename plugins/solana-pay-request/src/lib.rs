#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfoGuest;
    use exports::zeroclaw::plugin::tool::{Guest as ToolGuest, ToolResult};

    use serde::Deserialize;

    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct Component;

    const PLUGIN_NAME: &str = "solana-pay-request";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

    #[derive(Deserialize)]
    struct InputArgs {
        #[serde(default)]
        amount_brl: Option<serde_json::Value>,

        #[serde(default)]
        amount: Option<serde_json::Value>,

        #[serde(default)]
        valor: Option<serde_json::Value>,

        #[serde(default)]
        valor_brl: Option<serde_json::Value>,

        #[serde(default)]
        memo: Option<String>,

        #[serde(default)]
        text: Option<String>,

        #[serde(default)]
        message: Option<String>,

        #[serde(default)]
        prompt: Option<String>,

        #[serde(default)]
        query: Option<String>,

        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    impl InputArgs {
        /// Tenta extrair o valor cobrado em BRL, aceitando várias formas que
        /// o modelo pode usar (campo estruturado ou texto solto contendo o
        /// número), sem nunca aceitar um endereço de destinatário.
        fn parse_amount(&self) -> Result<f64, String> {
            if let Some(value) = self
                .amount_brl
                .as_ref()
                .or(self.amount.as_ref())
                .or(self.valor_brl.as_ref())
                .or(self.valor.as_ref())
            {
                return parse_money(value);
            }

            let text = self
                .text
                .as_ref()
                .or(self.message.as_ref())
                .or(self.prompt.as_ref())
                .or(self.query.as_ref());

            if let Some(text) = text {
                if let Some(value) = extract_first_number(text) {
                    return Ok(value);
                }
            }

            Err("nenhum valor encontrado".into())
        }

        fn parse_memo(&self) -> String {
            if let Some(m) = &self.memo {
                return m.clone();
            }

            self.text
                .as_ref()
                .or(self.message.as_ref())
                .or(self.prompt.as_ref())
                .or(self.query.as_ref())
                .cloned()
                .unwrap_or_default()
        }
    }

    /// Extrai o primeiro número (aceitando vírgula ou ponto decimal) de um
    /// texto solto, usado como fallback quando o modelo manda a frase inteira
    /// em vez de um campo estruturado.
    fn extract_first_number(text: &str) -> Option<f64> {
        let mut current = String::new();

        for c in text.chars() {
            if c.is_ascii_digit() || c == ',' || c == '.' {
                current.push(c);
            } else if !current.is_empty() {
                break;
            }
        }

        if current.is_empty() {
            return None;
        }

        current.replace(',', ".").parse::<f64>().ok()
    }

    /// Normaliza um valor de JSON (número ou string tipo "R$ 25,00") em f64.
    fn parse_money(value: &serde_json::Value) -> Result<f64, String> {
        match value {
            serde_json::Value::Number(n) => n.as_f64().ok_or("valor inválido".to_string()),
            serde_json::Value::String(s) => {
                let cleaned = s
                    .replace("R$", "")
                    .replace("reais", "")
                    .replace("real", "")
                    .replace(' ', "")
                    .replace(',', ".");

                cleaned.parse::<f64>().map_err(|_| "valor inválido".to_string())
            }
            _ => Err("valor inválido".to_string()),
        }
    }

    impl PluginInfoGuest for Component {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }

        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl ToolGuest for Component {
        fn name() -> String {
            PLUGIN_NAME.to_string()
        }

        fn description() -> String {
            "Gera um QR Code Solana Pay.\n\n\
             Sempre envie o valor em BRL.\n\n\
             Exemplo:\n\n\
             Usuário:\n\
             \"Cobrar 25 reais da mesa 4\"\n\n\
             Tool:\n\n\
             {\n\
             \"amount_brl\":25,\n\
             \"memo\":\"mesa 4\"\n\
             }\n\n\
             NUNCA converta BRL para SOL.\n\
             O plugin faz isso automaticamente."
                .to_string()
        }

        fn parameters_schema() -> String {
            r#"{
"type":"object",
"properties":{
"amount_brl":{
"type":"number",
"description":"Valor da cobrança em reais (BRL)"
},
"memo":{
"type":"string",
"description":"Identificação da cobrança"
}
},
"required":["amount_brl","memo"]
}"#
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let parsed: InputArgs = match serde_json::from_str(&args) {
                Ok(v) => v,
                Err(e) => {
                    let msg = format!("argumentos inválidos: {}", e);
                    emit(PluginAction::Fail, PluginOutcome::Failure, &msg);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(msg),
                    });
                }
            };

            let wallet = match parsed.config.get("pos_wallet").filter(|w| !w.trim().is_empty()) {
                Some(w) => w.clone(),
                None => {
                    let msg = "config ausente: pos_wallet".to_string();
                    emit(PluginAction::Read, PluginOutcome::Failure, &msg);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(msg),
                    });
                }
            };

            let brl_per_sol = parsed
                .config
                .get("brl_per_sol")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(pos_core::BRL_PER_SOL);

            let amount_brl = match parsed.parse_amount() {
                Ok(v) => v,
                Err(e) => {
                    emit(PluginAction::Validate, PluginOutcome::Failure, &e);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    });
                }
            };

            let memo = parsed.parse_memo();

            let request_brl = pos_core::PaymentRequestBrl {
                recipient: wallet,
                amount_brl,
                brl_per_sol,
                memo,
                label: Some("ZeroClaw POS Terminal".to_string()),
            };

            let request = match pos_core::PaymentRequest::try_from(request_brl) {
                Ok(v) => v,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, &e);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    });
                }
            };

            match pos_core::create_solana_pay_url(&request) {
                Ok(response) => {
                    emit(PluginAction::Complete, PluginOutcome::Success, "QR gerado");
                    Ok(ToolResult {
                        success: true,
                        output: serde_json::to_string_pretty(&response).unwrap(),
                        error: None,
                    })
                }
                Err(e) => {
                    emit(PluginAction::Complete, PluginOutcome::Failure, &e);
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    })
                }
            }
        }
    }

    fn emit(action: PluginAction, outcome: PluginOutcome, message: &str) {
        log_record(
            LogLevel::Info,
            &PluginEvent {
                function_name: "solana_pay_request::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(Component);
}