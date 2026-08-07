pub mod transaction;

use serde::{Deserialize, Serialize};

/// Taxa de conversão utilizada pelo POS.
/// 800 BRL = 1 SOL
pub const BRL_PER_SOL: f64 = 800.0;

/// Converte BRL para SOL arredondando para 3 casas decimais.
pub fn brl_to_sol(
    brl: f64,
    brl_per_sol: f64,
) -> Result<f64,String>
{
    if !brl.is_finite() {
        return Err("Valor inválido.".into());
    }

    if brl <= 0.0 {
        return Err("Valor deve ser maior que zero.".into());
    }

    if brl_per_sol <= 0.0 {
        return Err("Cotação inválida.".into());
    }

    let sol = brl / brl_per_sol;

    Ok((sol * 1000.0).round() / 1000.0)
}
/// Estrutura interna utilizada pelo protocolo Solana Pay.
/// Sempre utiliza SOL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub recipient: String,
    pub amount: f64,
    pub memo: String,
    pub label: Option<String>,
}

/// Estrutura utilizada pelo plugin.
/// Recebe BRL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequestBrl {
    pub recipient: String,
    pub amount_brl: f64,
    pub brl_per_sol: f64,
    pub memo: String,
    pub label: Option<String>,
}

impl TryFrom<PaymentRequestBrl> for PaymentRequest {
    type Error = String;

    fn try_from(req: PaymentRequestBrl) -> Result<Self, Self::Error> {
        Ok(Self {
            recipient: req.recipient,
            amount: brl_to_sol(
                req.amount_brl,
                req.brl_per_sol,
            )?,
            memo: req.memo,
            label: req.label,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub url: String,
    pub recipient: String,
    pub amount: f64,
    pub memo: String,
}

pub fn create_solana_pay_url(
    req: &PaymentRequest,
) -> Result<PaymentResponse, String> {
    if req.amount <= 0.0 {
        return Err("O valor da cobrança deve ser maior que zero.".to_string());
    }

    if req.recipient.trim().is_empty() {
        return Err("Endereço do destinatário não pode ser vazio.".to_string());
    }

    let memo_encoded = urlencoding::encode(&req.memo);

    let label_encoded = req
        .label
        .as_ref()
        .map(|l| format!("&label={}", urlencoding::encode(l)))
        .unwrap_or_default();

    let amount = format!("{:.3}", req.amount);

    let url = format!(
        "solana:{}?amount={}&memo={}{}",
        req.recipient,
        amount,
        memo_encoded,
        label_encoded
    );

    Ok(PaymentResponse {
        url,
        recipient: req.recipient.clone(),
        amount: req.amount,
        memo: req.memo.clone(),
    })
}

#[cfg(test)]
mod tests {

    use super::*;

   #[test]
fn test_800_brl() {
    assert_eq!(brl_to_sol(800.0, BRL_PER_SOL).unwrap(), 1.000);
}

#[test]
fn test_25_brl() {
    assert_eq!(brl_to_sol(25.0, BRL_PER_SOL).unwrap(), 0.031);
}

#[test]
fn test_100_brl() {
    assert_eq!(brl_to_sol(100.0, BRL_PER_SOL).unwrap(), 0.125);
}

#[test]
fn test_rounding() {
    assert_eq!(brl_to_sol(1.0, BRL_PER_SOL).unwrap(), 0.001);
}

#[test]
fn test_zero() {
    assert!(brl_to_sol(0.0, BRL_PER_SOL).is_err());
}

#[test]
fn test_negative() {
    assert!(brl_to_sol(-10.0, BRL_PER_SOL).is_err());
}

    #[test]
    fn test_url_generation() {

        let req = PaymentRequest {
            recipient: "ABCDE".to_string(),
            amount: 1.000,
            memo: "mesa 2".to_string(),
            label: Some("ZeroClaw POS".to_string()),
        };

        let url = create_solana_pay_url(&req).unwrap();

        assert!(url.url.contains("amount=1.000"));
        assert!(url.url.contains("mesa%202"));
    }
}