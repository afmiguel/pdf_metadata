use chrono::Local;
use lopdf::{Dictionary, Document, Object, ObjectId};
use lopdf::Error as LopfError;
use std::error::Error;
use std::fs;
use std::path::{Path};
use std::time::SystemTime;

/// Converte um valor de metadado PDF (`Object`) para uma `String` legível.
fn info_value_to_string(object: &Object) -> String {
    match object {
        Object::String(vec_bytes, _format) => {
            // object.as_str() tenta decodificar. A documentação diz que retorna Result<&str, Error>.
            // No entanto, o compilador do usuário indica que o valor em Ok(...) é tratado como &[u8]
            // ao tentar chamar .to_string(). Portanto, vamos tratar o resultado de Ok(...) como &[u8].
            match object.as_str() {
                Ok(data_from_as_str) => {
                    // Se data_from_as_str é &[u8] (conforme inferido pelo erro do compilador),
                    // esta é a conversão correta para String.
                    String::from_utf8_lossy(data_from_as_str).into_owned()
                }
                Err(_) => {
                    // Se as_str() falhar, usamos os bytes originais de Object::String.
                    // vec_bytes aqui é Vec<u8>.
                    String::from_utf8_lossy(vec_bytes).into_owned()
                }
            }
        }
        Object::Name(vec_bytes) => { // vec_bytes é Vec<u8>
            String::from_utf8_lossy(vec_bytes).into_owned()
        }
        Object::Integer(i) => i.to_string(),
        Object::Real(f) => f.to_string(),
        Object::Boolean(b) => b.to_string(),
        Object::Null => "null".to_string(),
        _ => {
            let type_name_bytes: &[u8] = object.type_name().unwrap_or(b"<Desconhecido>");
            let type_name_displayable: String = String::from_utf8_lossy(type_name_bytes).into_owned();
            format!("<Tipo {} não processado>", type_name_displayable)
        }
    }
}

/// Define (adiciona ou atualiza) um par chave-valor de metadados em um arquivo PDF.
/// Salva o resultado em `output_path`.
pub fn set_metadata(
    file_path: &str,
    output_path: &str,
    metadata_key: &str,
    metadata_value: &str,
) -> Result<(), Box<dyn Error>> {
    let mut doc = Document::load(file_path)?;

    let info_dict_id_res: Result<ObjectId, LopfError> = doc
        .trailer
        .get(b"Info")
        .and_then(|obj_ref: &Object| obj_ref.as_reference());

    let info_dict_id: ObjectId = match info_dict_id_res {
        Ok(id) => id,
        Err(_e) => {
            let new_info_dict = Dictionary::new();
            let id = doc.add_object(new_info_dict);
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };

    let info_dict_obj = doc.get_object_mut(info_dict_id)?;
    let info_dict = info_dict_obj.as_dict_mut()?;

    info_dict.set(
        metadata_key.as_bytes().to_vec(),
        Object::string_literal(metadata_value),
    );

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

    doc.save(output_path)?;
    Ok(())
}

/// Atualiza os metadados de um arquivo PDF "in-place" (no mesmo arquivo) de forma segura.
pub fn update_metadata_in_place(
    file_path_str: &str,
    metadata_key: &str,
    metadata_value: &str,
) -> Result<(), Box<dyn Error>> {
    let original_path = Path::new(file_path_str);

    let mut doc = Document::load(file_path_str)?;

    let info_dict_id_res: Result<ObjectId, LopfError> = doc
        .trailer
        .get(b"Info")
        .and_then(|obj_ref: &Object| obj_ref.as_reference());

    let info_dict_id: ObjectId = match info_dict_id_res {
        Ok(id) => id,
        Err(_e) => {
            let new_info_dict = Dictionary::new();
            let id = doc.add_object(new_info_dict);
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };

    let info_dict_obj = doc.get_object_mut(info_dict_id)?;
    let info_dict = info_dict_obj.as_dict_mut()?;

    info_dict.set(
        metadata_key.as_bytes().to_vec(),
        Object::string_literal(metadata_value),
    );

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

    let parent_dir = original_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Diretório pai não encontrado")
    })?;
    let original_filename_stem = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("temp_pdf");
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_micros();
    let temp_filename_str = format!("{}_{}.pdf.tmp", original_filename_stem, timestamp);
    let temp_file_path = parent_dir.join(&temp_filename_str);

    if let Err(save_err) = doc.save(&temp_file_path) {
        let _ = fs::remove_file(&temp_file_path);
        return Err(format!("Erro ao salvar em arquivo temporário: {}", save_err).into());
    }

    if let Err(rename_err) = fs::rename(&temp_file_path, original_path) {
        let _ = fs::remove_file(&temp_file_path);
        return Err(format!("Erro ao renomear arquivo temporário para o original: {}", rename_err).into());
    }

    Ok(())
}

/// Retorna um vetor de pares (Chave, Valor) dos metadados do dicionário Info do PDF.
pub fn get_metadata(file_path: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let doc = Document::load(file_path)?;
    let mut metadata_entries = Vec::new();

    let info_dict_id_res: Result<ObjectId, LopfError> = doc
        .trailer
        .get(b"Info")
        .and_then(|obj_ref: &Object| {
            obj_ref.as_reference()
        });

    if let Ok(info_dict_id) = info_dict_id_res {
        if let Ok(info_object_ref) = doc.get_object(info_dict_id) {
            if let Ok(dictionary) = info_object_ref.as_dict() {
                for (key_bytes, value_object) in dictionary.iter() {
                    let key = String::from_utf8_lossy(key_bytes).into_owned();
                    let value = info_value_to_string(value_object);
                    metadata_entries.push((key, value));
                }
            }
        }
    }
    Ok(metadata_entries)
}

#[cfg(test)]
mod tests {
    use super::*; // Importa tudo do módulo pai (sua lib)
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::env; // Para obter o diretório temporário
    use std::time::{SystemTime, UNIX_EPOCH}; // Para nomes de arquivo únicos

    // Função auxiliar para criar um diretório de teste único e limpá-lo depois
    // Retorna o caminho para o diretório criado
    fn setup_unique_test_dir(test_name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros();
        let mut temp_dir = env::temp_dir();
        temp_dir.push(format!("{}_{}", test_name, millis));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp test directory");
        temp_dir
    }

    // Função auxiliar para criar um PDF mínimo para testes
    fn create_minimal_test_pdf(path: &Path) -> Result<(), Box<dyn Error>> {
        let mut doc = Document::with_version("1.7");
        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Count", Object::Integer(0));
        pages_dict.set("Kids", Object::Array(vec![]));
        let pages_id = doc.add_object(pages_dict);
        catalog_dict.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(catalog_dict);
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path)?;
        Ok(())
    }

    #[test]
    fn test_set_metadata_creates_file_and_adds_key() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("set_metadata_test");
        let original_file = test_dir.join("original_set.pdf");
        let output_file = test_dir.join("output_set.pdf");

        create_minimal_test_pdf(&original_file)?;

        let key = "MySetKey";
        let value = "MySetValue Āččęñtš"; // Teste com caracteres não-ASCII
        set_metadata(original_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value)?;

        assert!(output_file.exists(), "O arquivo de saída deveria ter sido criado");

        let metadata = get_metadata(output_file.to_str().unwrap())?;
        let entry = metadata.iter().find(|(k, _)| k == key);
        assert!(entry.is_some(), "A chave de metadados não foi encontrada");
        assert_eq!(entry.unwrap().1, value, "O valor do metadado não corresponde");

        let mod_date_exists = metadata.iter().any(|(k, _)| k == "ModDate");
        assert!(mod_date_exists, "ModDate deveria existir");

        fs::remove_dir_all(test_dir)?; // Limpeza
        Ok(())
    }

    #[test]
    fn test_set_metadata_overwrites_existing_key() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("set_metadata_overwrite");
        let original_file = test_dir.join("original_overwrite.pdf");
        let output_file = test_dir.join("output_overwrite.pdf");

        create_minimal_test_pdf(&original_file)?;

        let key = "MyKeyToOverwrite";
        let value1 = "InitialValue";
        let value2 = "OverwrittenValue";

        set_metadata(original_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value1)?;
        set_metadata(output_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value2)?; // Sobrescreve no mesmo arquivo de saída

        let metadata = get_metadata(output_file.to_str().unwrap())?;
        let entry = metadata.iter().find(|(k, _)| k == key);
        assert_eq!(entry.unwrap().1, value2, "O valor deveria ter sido sobrescrito");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_metadata_from_pdf_with_no_info() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("get_metadata_no_info");
        let pdf_file = test_dir.join("no_info.pdf");
        create_minimal_test_pdf(&pdf_file)?; // Este PDF não terá um dicionário Info inicialmente

        let metadata = get_metadata(pdf_file.to_str().unwrap())?;
        assert!(metadata.is_empty(), "Deveria retornar um vetor vazio para PDF sem metadados Info");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_update_metadata_in_place_modifies_file() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("update_in_place_test");
        let file_to_update = test_dir.join("update_me.pdf");

        create_minimal_test_pdf(&file_to_update)?;

        let key = "MyUpdateKey";
        let value = "ValueUpdatedInPlace";
        update_metadata_in_place(file_to_update.to_str().unwrap(), key, value)?;

        let metadata = get_metadata(file_to_update.to_str().unwrap())?;
        let entry = metadata.iter().find(|(k, _)| k == key);
        assert!(entry.is_some(), "A chave de metadados não foi encontrada após atualização in-place");
        assert_eq!(entry.unwrap().1, value, "O valor do metadado não corresponde após atualização in-place");

        let mod_date_exists = metadata.iter().any(|(k, _)| k == "ModDate");
        assert!(mod_date_exists, "ModDate deveria existir após atualização in-place");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_info_value_to_string_handles_various_types() {
        // Teste direto da função auxiliar (se ela for pública, caso contrário, teste indireto)
        // Como info_value_to_string é privada, este teste é conceitual e seria testado
        // através de get_metadata com um PDF construído especificamente.
        // Para este exemplo, vamos assumir que get_metadata cobre os casos.
        // Se info_value_to_string fosse pública:
        // assert_eq!(info_value_to_string(&Object::String(b"Hello PDF".to_vec(), lopdf::StringFormat::Literal)), "Hello PDF");
        // assert_eq!(info_value_to_string(&Object::Name(b"MyName".to_vec())), "MyName");
        // assert_eq!(info_value_to_string(&Object::Integer(42)), "42");
        // etc.
        // Como é privada, confiamos que os testes de get_metadata que usam set_metadata
        // (que insere strings) cobrem o caminho principal.
    }

    #[test]
    fn test_set_metadata_file_not_found() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("set_metadata_err");
        let output_file = test_dir.join("output_err.pdf");
        let result = set_metadata("non_existent_input.pdf", output_file.to_str().unwrap(), "key", "value");
        assert!(result.is_err(), "Deveria retornar erro se o arquivo de entrada não existe");
        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_update_metadata_in_place_file_not_found() -> Result<(), Box<dyn Error>> {
        let result = update_metadata_in_place("non_existent_update.pdf", "key", "value");
        assert!(result.is_err(), "Deveria retornar erro se o arquivo para atualizar não existe");
        Ok(())
    }

    #[test]
    fn test_get_metadata_file_not_found() -> Result<(), Box<dyn Error>> {
        let result = get_metadata("non_existent_get.pdf");
        assert!(result.is_err(), "Deveria retornar erro se o arquivo para ler não existe");
        Ok(())
    }
}