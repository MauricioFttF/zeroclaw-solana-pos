use std::fmt;

/// Escreve um comprimento no formato compact-u16 (shortvec) que a Solana usa
/// para prefixar vetores em transações — NÃO é o mesmo formato que o borsh usa.
fn write_shortvec_len(buf: &mut Vec<u8>, mut len: usize) {
    loop {
        let mut byte = (len & 0x7f) as u8;
        len >>= 7;
        if len != 0 {
            byte |= 0x80;
            buf.push(byte);
        } else {
            buf.push(byte);
            break;
        }
    }
}

/// Representa os exatos 32 bytes de uma chave pública na Solana
#[derive(Clone, PartialEq)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Tenta converter uma string Base58 em um array de 32 bytes
    pub fn from_str(s: &str) -> Result<Self, String> {
        let decoded = bs58::decode(s)
            .into_vec()
            .map_err(|e| format!("Erro ao decodificar Base58: {}", e))?;

        if decoded.len() != 32 {
            return Err("Tamanho inválido para uma Pubkey da Solana".to_string());
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Pubkey(bytes))
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

/// A estrutura básica de uma Instrução que a Solana consegue executar
#[derive(Debug, Clone)]
pub struct Instruction {
    /// O ID do programa que vai ser executado (ex: System Program)
    pub program_id: Pubkey,
    /// As contas que essa instrução vai ler ou escrever
    pub accounts: Vec<AccountMeta>,
    /// O "opcode" e os parâmetros da instrução compactados em bytes
    pub data: Vec<u8>,
}

/// Define os privilégios de uma conta dentro da instrução
#[derive(Debug, Clone)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl Instruction {
    /// 1. Constrói a instrução para avançar o Durable Nonce
    pub fn new_advance_nonce(
        nonce_account: Pubkey,
        nonce_authority: Pubkey,
    ) -> Result<Self, String> {
        let system_program = Pubkey::from_str("11111111111111111111111111111111")?;
        let sysvar_recent_blockhashes =
            Pubkey::from_str("SysvarRecentB1ockHashes11111111111111111111")?;

        let accounts = vec![
            AccountMeta { pubkey: nonce_account, is_signer: false, is_writable: true },
            AccountMeta { pubkey: sysvar_recent_blockhashes, is_signer: false, is_writable: false },
            AccountMeta { pubkey: nonce_authority, is_signer: true, is_writable: false },
        ];

        // Opcode 4 = AdvanceNonceAccount (u32 little-endian)
        let data = vec![4, 0, 0, 0];

        Ok(Instruction { program_id: system_program, accounts, data })
    }

    /// 2. Constrói uma transferência de tokens SPL usando TransferChecked
    /// (opcode 12), que exige o mint e as decimais explicitamente — evita
    /// erro de casas decimais que o Transfer simples (opcode 3) permite.
    pub fn new_spl_transfer(
        source: Pubkey,
        mint: Pubkey,
        destination: Pubkey,
        owner: Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Self, String> {
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;

        let accounts = vec![
            AccountMeta { pubkey: source, is_signer: false, is_writable: true },
            AccountMeta { pubkey: mint, is_signer: false, is_writable: false },
            AccountMeta { pubkey: destination, is_signer: false, is_writable: true },
            AccountMeta { pubkey: owner, is_signer: true, is_writable: false },
        ];

        // Opcode 12 = TransferChecked
        let mut data = vec![12u8];
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(decimals);

        Ok(Instruction { program_id: token_program, accounts, data })
    }
}

/// Cabeçalho obrigatório que diz à rede quantas assinaturas esperar
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
}

/// A instrução "compilada", onde as chaves públicas viram apenas índices (u8)
#[derive(Debug, Clone)]
pub struct CompiledInstruction {
    /// Índice do programa (ex: Token Program) no array de account_keys
    pub program_id_index: u8,
    /// Vetor com os índices das contas exigidas por esta instrução
    pub accounts: Vec<u8>,
    /// O opcode e os parâmetros em bytes
    pub data: Vec<u8>,
}

/// O payload final que será serializado, assinado e enviado para a blockchain
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    /// O array "mestre" contendo todas as chaves únicas usadas na transação
    pub account_keys: Vec<Pubkey>,
    /// O hash do bloco recente ou o bloco congelado do nosso Durable Nonce
    pub recent_blockhash: Pubkey,
    pub instructions: Vec<CompiledInstruction>,
}

impl Message {
    /// Compila instruções de alto nível no formato de índices da Solana,
    /// com as contas ordenadas nos 4 grupos exigidos pelo protocolo:
    /// signer+writable -> signer+readonly -> non-signer+writable -> non-signer+readonly
    pub fn new(instructions: &[Instruction], payer: &Pubkey, recent_blockhash: Pubkey) -> Self {
        // Agrega os metadados (signer/writable) de cada pubkey única.
        // O payer é sempre signer+writable por definição.
        let mut meta: Vec<(Pubkey, bool, bool)> = vec![(payer.clone(), true, true)];

        let mut upsert = |pubkey: Pubkey, is_signer: bool, is_writable: bool| {
            if let Some(entry) = meta.iter_mut().find(|(k, _, _)| k == &pubkey) {
                entry.1 |= is_signer;
                entry.2 |= is_writable;
            } else {
                meta.push((pubkey, is_signer, is_writable));
            }
        };

        for ix in instructions {
            upsert(ix.program_id.clone(), false, false);
            for acc in &ix.accounts {
                upsert(acc.pubkey.clone(), acc.is_signer, acc.is_writable);
            }
        }

        let payer_key = payer.clone();
        let mut signer_writable: Vec<Pubkey> = vec![payer_key.clone()];
        let mut signer_readonly: Vec<Pubkey> = vec![];
        let mut nonsigner_writable: Vec<Pubkey> = vec![];
        let mut nonsigner_readonly: Vec<Pubkey> = vec![];

        for (key, is_signer, is_writable) in meta {
            if key == payer_key {
                continue;
            }
            match (is_signer, is_writable) {
                (true, true) => signer_writable.push(key),
                (true, false) => signer_readonly.push(key),
                (false, true) => nonsigner_writable.push(key),
                (false, false) => nonsigner_readonly.push(key),
            }
        }

        let num_required_signatures = (signer_writable.len() + signer_readonly.len()) as u8;
        let num_readonly_signed_accounts = signer_readonly.len() as u8;
        let num_readonly_unsigned_accounts = nonsigner_readonly.len() as u8;

        let mut account_keys = signer_writable;
        account_keys.extend(signer_readonly);
        account_keys.extend(nonsigner_writable);
        account_keys.extend(nonsigner_readonly);

        let compiled_instructions: Vec<CompiledInstruction> = instructions
            .iter()
            .map(|ix| {
                let program_id_index =
                    account_keys.iter().position(|k| k == &ix.program_id).unwrap() as u8;
                let accounts = ix
                    .accounts
                    .iter()
                    .map(|meta| account_keys.iter().position(|k| k == &meta.pubkey).unwrap() as u8)
                    .collect();

                CompiledInstruction { program_id_index, accounts, data: ix.data.clone() }
            })
            .collect();

        let header = MessageHeader {
            num_required_signatures,
            num_readonly_signed_accounts,
            num_readonly_unsigned_accounts,
        };

        Message { header, account_keys, recent_blockhash, instructions: compiled_instructions }
    }

    /// Serializa a mensagem no formato binário real da Solana (compact-u16),
    /// não em borsh. Retorna a string em Base58 pronta pra ir numa transação.
    pub fn serialize_to_base58(&self) -> Result<String, String> {
        let mut buf = Vec::new();

        buf.push(self.header.num_required_signatures);
        buf.push(self.header.num_readonly_signed_accounts);
        buf.push(self.header.num_readonly_unsigned_accounts);

        write_shortvec_len(&mut buf, self.account_keys.len());
        for key in &self.account_keys {
            buf.extend_from_slice(&key.0);
        }

        buf.extend_from_slice(&self.recent_blockhash.0);

        write_shortvec_len(&mut buf, self.instructions.len());
        for ix in &self.instructions {
            buf.push(ix.program_id_index);

            write_shortvec_len(&mut buf, ix.accounts.len());
            buf.extend_from_slice(&ix.accounts);

            write_shortvec_len(&mut buf, ix.data.len());
            buf.extend_from_slice(&ix.data);
        }

        Ok(bs58::encode(buf).into_string())
    }
}