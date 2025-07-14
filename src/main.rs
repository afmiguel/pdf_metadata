use pdf_metadata::{get_metadata, update_metadata_in_place};
use dialoguer::{Select, Input, Confirm};
use lopdf::{Document, Object};
use std::env;
use std::process;
use std::error::Error;
use std::fs;
use chrono::Local;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Uso: {} <caminho_para_arquivo.pdf>", args[0]);
        eprintln!("Exemplo: {} /caminho/para/documento.pdf", args[0]);
        process::exit(1);
    }

    let pdf_path = &args[1];
    
    if !std::path::Path::new(pdf_path).exists() {
        eprintln!("Erro: Arquivo não encontrado: {}", pdf_path);
        process::exit(1);
    }

    println!("\n📄 Editor de Metadados PDF");
    println!("Arquivo: {}", pdf_path);
    println!("{}", "═".repeat(60));

    loop {
        match show_main_menu(pdf_path) {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(e) => {
                eprintln!("❌ Erro: {}", e);
                if atty::is(atty::Stream::Stdin) {
                    let retry = Confirm::new()
                        .with_prompt("Deseja tentar novamente?")
                        .default(true)
                        .interact()
                        .unwrap_or(false);
                    if !retry {
                        break;
                    }
                } else {
                    eprintln!("Executando em modo não-interativo. Saindo...");
                    break;
                }
            }
        }
    }
    
    println!("\n👋 Obrigado por usar o Editor de Metadados PDF!");
}

fn show_main_menu(pdf_path: &str) -> Result<bool, Box<dyn Error>> {
    // Verifica se está rodando em terminal interativo
    if !atty::is(atty::Stream::Stdin) {
        // Se não for interativo, apenas lista os metadados e sai
        list_metadata(pdf_path)?;
        return Ok(false);
    }

    let options = vec![
        "📋 Listar todos os metadados",
        "➕ Criar novo metadado", 
        "✏️  Editar valor de metadado",
        "🔄 Alterar chave de metadado",
        "🗑️  Excluir metadado",
        "🚪 Sair"
    ];

    let selection = Select::new()
        .with_prompt("\nSelecione uma opção:")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            list_metadata(pdf_path)?;
            wait_for_enter();
        }
        1 => create_metadata(pdf_path)?,
        2 => edit_metadata_value(pdf_path)?,
        3 => change_metadata_key(pdf_path)?,
        4 => delete_metadata(pdf_path)?,
        5 => return Ok(false),
        _ => unreachable!()
    }
    
    Ok(true)
}

fn list_metadata(pdf_path: &str) -> Result<(), Box<dyn Error>> {
    println!("\n📋 Metadados do PDF:");
    println!("{}", "─".repeat(50));
    
    let metadata = get_metadata(pdf_path)?;
    
    if metadata.is_empty() {
        println!("ℹ️  Nenhum metadado encontrado.");
        return Ok(());
    }
    
    for (i, (key, value)) in metadata.iter().enumerate() {
        let display_value = if value.len() > 60 {
            format!("{}...", &value[..57])
        } else {
            value.clone()
        };
        
        println!("{:2}. {:<20}: {}", i + 1, key, display_value);
    }
    
    println!("\n📊 Total: {} metadados", metadata.len());
    Ok(())
}

fn create_metadata(pdf_path: &str) -> Result<(), Box<dyn Error>> {
    println!("\n➕ Criar Novo Metadado");
    println!("{}", "─".repeat(30));
    
    let existing_metadata = get_metadata(pdf_path)?;
    
    let key: String = loop {
        let input_key = Input::<String>::new()
            .with_prompt("Chave do metadado")
            .interact_text()?;
            
        if input_key.trim().is_empty() {
            println!("⚠️  A chave não pode estar vazia.");
            continue;
        }
        
        if existing_metadata.iter().any(|(k, _)| k == &input_key) {
            println!("⚠️  A chave '{}' já existe. Use a opção de editar.", input_key);
            continue;
        }
        
        break input_key;
    };
    
    let value = Input::<String>::new()
        .with_prompt("Valor do metadado")
        .allow_empty(true)
        .interact_text()?;
        
    let has_accents = value.chars().any(|c| !c.is_ascii());
    let use_base64 = if has_accents {
        Confirm::new()
            .with_prompt("Detectados caracteres não-ASCII. Usar codificação BASE64?")
            .default(true)
            .interact()?
    } else {
        false
    };
    
    let final_value = if use_base64 {
        encode_to_base64_utf16be(&value)
    } else {
        value
    };
    
    update_metadata_in_place(pdf_path, &key, &final_value)?;
    println!("✅ Metadado '{}' criado com sucesso!", key);
    
    Ok(())
}

fn edit_metadata_value(pdf_path: &str) -> Result<(), Box<dyn Error>> {
    println!("\n✏️  Editar Valor de Metadado");
    println!("{}", "─".repeat(35));
    
    let metadata = get_metadata(pdf_path)?;
    
    if metadata.is_empty() {
        println!("ℹ️  Nenhum metadado encontrado para editar.");
        return Ok(());
    }
    
    let keys: Vec<String> = metadata.iter().map(|(k, _)| k.clone()).collect();
    
    let selection = Select::new()
        .with_prompt("Selecione o metadado para editar")
        .items(&keys)
        .interact()?;
        
    let selected_key = &keys[selection];
    let current_value = &metadata[selection].1;
    
    println!("\nChave: {}", selected_key);
    println!("Valor atual: {}", current_value);
    
    let new_value = Input::<String>::new()
        .with_prompt("Novo valor")
        .with_initial_text(current_value)
        .interact_text()?;
        
    let has_accents = new_value.chars().any(|c| !c.is_ascii());
    let use_base64 = if has_accents {
        Confirm::new()
            .with_prompt("Detectados caracteres não-ASCII. Usar codificação BASE64?")
            .default(true)
            .interact()?
    } else {
        false
    };
    
    let final_value = if use_base64 {
        encode_to_base64_utf16be(&new_value)
    } else {
        new_value
    };
        
    update_metadata_in_place(pdf_path, selected_key, &final_value)?;
    println!("✅ Valor do metadado '{}' atualizado com sucesso!", selected_key);
    
    Ok(())
}

fn change_metadata_key(pdf_path: &str) -> Result<(), Box<dyn Error>> {
    println!("\n🔄 Alterar Chave de Metadado");
    println!("{}", "─".repeat(35));
    
    let metadata = get_metadata(pdf_path)?;
    
    if metadata.is_empty() {
        println!("ℹ️  Nenhum metadado encontrado para alterar.");
        return Ok(());
    }
    
    let keys: Vec<String> = metadata.iter().map(|(k, _)| k.clone()).collect();
    
    let selection = Select::new()
        .with_prompt("Selecione o metadado para alterar a chave")
        .items(&keys)
        .interact()?;
        
    let old_key = &keys[selection];
    let value = &metadata[selection].1;
    
    println!("\nChave atual: {}", old_key);
    
    let new_key: String = loop {
        let input_key = Input::<String>::new()
            .with_prompt("Nova chave")
            .with_initial_text(old_key)
            .interact_text()?;
            
        if input_key.trim().is_empty() {
            println!("⚠️  A chave não pode estar vazia.");
            continue;
        }
        
        if input_key == *old_key {
            println!("⚠️  A nova chave deve ser diferente da atual.");
            continue;
        }
        
        if keys.contains(&input_key) {
            println!("⚠️  A chave '{}' já existe.", input_key);
            continue;
        }
        
        break input_key;
    };
    
    // Primeiro adiciona a nova chave
    update_metadata_in_place(pdf_path, &new_key, value)?;
    
    // Depois remove a chave antiga
    remove_metadata_key(pdf_path, old_key)?;
    
    println!("✅ Chave alterada de '{}' para '{}' com sucesso!", old_key, new_key);
    
    Ok(())
}

fn delete_metadata(pdf_path: &str) -> Result<(), Box<dyn Error>> {
    println!("\n🗑️  Excluir Metadado");
    println!("{}", "─".repeat(25));
    
    let metadata = get_metadata(pdf_path)?;
    
    if metadata.is_empty() {
        println!("ℹ️  Nenhum metadado encontrado para excluir.");
        return Ok(());
    }
    
    let keys: Vec<String> = metadata.iter().map(|(k, _)| k.clone()).collect();
    
    let selection = Select::new()
        .with_prompt("Selecione o metadado para excluir")
        .items(&keys)
        .interact()?;
        
    let selected_key = &keys[selection];
    let selected_value = &metadata[selection].1;
    
    println!("\nChave: {}", selected_key);
    println!("Valor: {}", selected_value);
    
    let confirm = Confirm::new()
        .with_prompt("Tem certeza que deseja excluir este metadado?")
        .default(false)
        .interact()?;
        
    if confirm {
        remove_metadata_key(pdf_path, selected_key)?;
        println!("✅ Metadado '{}' excluído com sucesso!", selected_key);
    } else {
        println!("❌ Operação cancelada.");
    }
    
    Ok(())
}

fn remove_metadata_key(pdf_path: &str, key_to_remove: &str) -> Result<(), Box<dyn Error>> {
    let mut doc = Document::load(pdf_path)?;
    
    let info_dict_id = doc
        .trailer
        .get(b"Info")
        .and_then(|obj_ref| obj_ref.as_reference())
        .map_err(|_| "PDF não possui dicionário Info")?;
        
    let info_dict_obj = doc.get_object_mut(info_dict_id)?;
    let info_dict = info_dict_obj.as_dict_mut()?;
    
    info_dict.remove(key_to_remove.as_bytes());
    
    // Atualiza ModDate
    let now = Local::now();
    let offset = now.offset();
    let offset_hours = offset.local_minus_utc() / 3600;
    let offset_minutes = (offset.local_minus_utc().abs() % 3600) / 60;
    let offset_sign = if offset.local_minus_utc() >= 0 { '+' } else { '-' };
    let pdf_date_formatted = format!(
        "D:{}{}{:02}'{:02}'",
        now.format("%Y%m%d%H%M%S"),
        offset_sign,
        offset_hours.abs(),
        offset_minutes
    );
    info_dict.set("ModDate", Object::string_literal(pdf_date_formatted));
    
    // Salva usando método temporário como nas outras funções
    let original_path = std::path::Path::new(pdf_path);
    let parent_dir = original_path.parent().ok_or("Não foi possível determinar diretório pai")?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_micros();
    let temp_filename = format!("temp_remove_{}_{}.pdf", 
        original_path.file_stem().unwrap().to_string_lossy(), timestamp);
    let temp_path = parent_dir.join(temp_filename);
    
    doc.save(&temp_path)?;
    fs::rename(&temp_path, pdf_path)?;
    
    Ok(())
}

fn encode_to_base64_utf16be(text: &str) -> String {
    let mut utf16_bytes = vec![0xFE, 0xFF]; // UTF-16BE BOM
    for ch in text.encode_utf16() {
        utf16_bytes.extend_from_slice(&ch.to_be_bytes());
    }
    
    let base64_encoded = base64_encode(&utf16_bytes);
    format!("UTF16BE:{}", base64_encoded)
}

fn base64_encode(input: &[u8]) -> String {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in input.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }
        
        let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        
        result.push(chars.chars().nth(((b >> 18) & 63) as usize).unwrap());
        result.push(chars.chars().nth(((b >> 12) & 63) as usize).unwrap());
        
        if chunk.len() > 1 {
            result.push(chars.chars().nth(((b >> 6) & 63) as usize).unwrap());
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(chars.chars().nth((b & 63) as usize).unwrap());
        } else {
            result.push('=');
        }
    }
    
    result
}

fn wait_for_enter() {
    println!("\n⏎ Pressione Enter para continuar...");
    let _ = std::io::stdin().read_line(&mut String::new());
}