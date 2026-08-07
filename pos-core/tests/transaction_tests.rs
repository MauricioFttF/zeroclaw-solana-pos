use pos_core::transaction::{Instruction, Pubkey, Message, MessageHeader, CompiledInstruction};

#[test]
fn valida_payload_spl_transfer() {
    // 1. Setup: Criamos as chaves públicas
    let source = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let dest = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let owner = Pubkey::from_str("11111111111111111111111111111111").unwrap();

    // 2. Execução: Simulamos a cobrança de 10 USDC (10_000_000 em casas decimais)
    let amount: u64 = 10_000_000; 
    let mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(); // mint da USDC
    let instruction = Instruction::new_spl_transfer(source, mint, dest, owner, amount, 6).unwrap();
    // 3. Verificação: opcode 12 (TransferChecked) + amount little-endian (8 bytes) + decimals (1 byte)
    let mut expected_data = vec![12u8];
    expected_data.extend_from_slice(&[0x80, 0x96, 0x98, 0x00, 0x00, 0x00, 0x00, 0x00]);
    expected_data.push(6u8);

    assert_eq!(
        instruction.data, 
        expected_data, 
        "Falha crítica: O alinhamento dos bytes da instrução não bate com o esperado pela SVM"
    );
}

#[test]
fn valida_estrutura_message() {
    // Simulando a montagem de um envelope simples
    let dummy_pubkey = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    
    let header = MessageHeader {
        num_required_signatures: 1,
        num_readonly_signed_accounts: 0,
        num_readonly_unsigned_accounts: 1,
    };

    let compiled_instruction = CompiledInstruction {
        program_id_index: 1, // Aponta para a segunda chave no array
        accounts: vec![0],   // Aponta para a primeira chave no array
        data: vec![3, 0, 0, 0],
    };

    let message = Message {
        header,
        account_keys: vec![dummy_pubkey.clone(), dummy_pubkey.clone()],
        recent_blockhash: dummy_pubkey, // O Nonce entra aqui
        instructions: vec![compiled_instruction],
    };

    assert_eq!(message.account_keys.len(), 2, "O array de chaves mestre deve conter os elementos passados");
    assert_eq!(message.instructions[0].program_id_index, 1, "O ponteiro de índice deve estar correto");
}

#[test]
fn valida_compilacao_e_serializacao_completa() {
    // 1. Mock de Contas
    let source = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let dest = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let owner = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let recent_blockhash = Pubkey::from_str("11111111111111111111111111111111").unwrap(); // Nonce congelado
    
    // 2. Criar a instrução de transferência de USDC
    let mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(); // mint da USDC
    let instruction = Instruction::new_spl_transfer(source, mint, dest, owner.clone(), 10_000_000, 6).unwrap();

    // 3. Compilar a Mensagem
    let message = Message::new(&[instruction], &owner, recent_blockhash);

    // 4. Gerar o Payload Final
    let base58_payload = message.serialize_to_base58().expect("Falha ao serializar com Borsh");

    // Validações
    assert!(base58_payload.len() > 50, "O payload Base58 deve ser uma string longa");
    println!("🎉 Payload Final do POS gerado com sucesso: {}", base58_payload);
}