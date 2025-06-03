//! # PDF Metadata Utility
//!
//! A Rust library for reading, setting, and updating metadata in PDF files.
//! This library allows you to interact with the PDF's "Info" dictionary,
//! where standard metadata fields like Author, Title, Subject, Keywords,
//! Creator, Producer, CreationDate, and ModDate are typically stored,
//! as well as custom metadata entries.
//!
//! ## Adding to Your Project
//!
//! To use this library in your Rust project, add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! pdf_metadata = { git = "[https://github.com/afmiguel/pdf_metadata.git](https://github.com/afmiguel/pdf_metadata.git)", branch = "master" }
//! # Or specify a tag/commit for stability, e.g., tag = "v0.1.0"
//! ```
//!
//! ## Examples
//!
//! Basic usage for getting metadata:
//!
//! ```no_run
//! use pdf_metadata::get_metadata;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     match get_metadata("path/to/your/document.pdf") {
//!         Ok(metadata_list) => {
//!             if metadata_list.is_empty() {
//!                 println!("No metadata found or Info dictionary is empty.");
//!             } else {
//!                 println!("PDF Metadata:");
//!                 for (key, value) in metadata_list {
//!                     println!("  {}: {}", key, value);
//!                 }
//!             }
//!         }
//!         Err(e) => {
//!             eprintln!("Error getting metadata: {}", e);
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use chrono::Local;
use lopdf::{Dictionary, Document, Object, ObjectId};
use lopdf::Error as LopfError;
use std::error::Error;
use std::fs;
use std::path::{Path};
use std::time::SystemTime;

/// Converts a PDF metadata `Object` value into a human-readable `String`.
///
/// This function handles various PDF object types that can be found in an Info dictionary,
/// attempting to provide the most sensible string representation.
///
/// # Arguments
///
/// * `object`: A reference to the `lopdf::Object` to be converted.
///
/// # Returns
///
/// A `String` representation of the PDF object. For complex or unhandled types,
/// it returns a placeholder string indicating the type.
fn info_value_to_string(object: &Object) -> String {
    match object {
        Object::String(vec_bytes, _format) => {
            // object.as_str() attempts to decode based on PDF string encoding rules.
            // The compiler's behavior in the user's environment suggested that
            // the Ok variant might be treated as &[u8] when .to_string() is called directly.
            // Thus, we consistently use from_utf8_lossy for safety.
            match object.as_str() {
                Ok(data_from_as_str) => {
                    // If data_from_as_str is &[u8] (as inferred from previous compiler errors),
                    // this is the correct conversion. If it's &str, from_utf8_lossy also works.
                    String::from_utf8_lossy(data_from_as_str).into_owned()
                }
                Err(_) => {
                    // If as_str() fails, use the original bytes from Object::String.
                    String::from_utf8_lossy(vec_bytes).into_owned()
                }
            }
        }
        Object::Name(vec_bytes) => { // vec_bytes is Vec<u8>
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

/// Sets (adds or updates) a specific metadata entry in a PDF file and saves it to a new path.
///
/// This function loads a PDF from `file_path`, modifies its Info dictionary
/// by adding or updating the `metadata_key` with `metadata_value`,
/// updates the `ModDate` field to the current time, and then saves the
/// modified document to `output_path`.
///
/// # Arguments
///
/// * `file_path`: The path to the original PDF file.
/// * `output_path`: The path where the modified PDF file will be saved.
/// * `metadata_key`: The key of the metadata entry to set (e.g., "Author", "MyCustomKey").
/// * `metadata_value`: The value for the metadata entry.
///
/// # Returns
///
/// * `Ok(())` if the operation was successful.
/// * `Err(Box<dyn Error>)` if any error occurs during loading, modification, or saving.
///
/// # Behavior
///
/// * If the `metadata_key` already exists, its value will be overwritten.
/// * If the PDF does not have an Info dictionary, one will be created.
/// * The `ModDate` field in the Info dictionary will be set to the current system time.
///
/// # Example
///
/// ```no_run
/// use pdf_metadata::set_metadata;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let original_pdf = "path/to/input.pdf";
///     let modified_pdf = "path/to/output_with_metadata.pdf";
///     let key = "Author";
///     let value = "Jane Doe";
///
///     match set_metadata(original_pdf, modified_pdf, key, value) {
///         Ok(_) => println!("Successfully set metadata and saved to {}", modified_pdf),
///         Err(e) => eprintln!("Error setting metadata: {}", e),
///     }
///     Ok(())
/// }
/// ```
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
        Err(_e) => { // If Info dictionary doesn't exist or is not a reference, create a new one.
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
        "D:{}{}{:02}'{:02}'", // PDF Date format e.g., D:20231027153000+02'00'
        now.format("%Y%m%d%H%M%S"),
        offset_sign,
        offset_hours.abs(),
        offset_minutes
    );
    info_dict.set("ModDate", Object::string_literal(pdf_date_formatted));

    doc.save(output_path)?;
    Ok(())
}

/// Updates a specific metadata entry in a PDF file "in-place" safely.
///
/// This function modifies the Info dictionary of the PDF specified by `file_path_str`
/// by adding or updating the `metadata_key` with `metadata_value`.
/// The `ModDate` field is also updated. The update is performed by first saving
/// to a temporary file in the same directory, and then replacing the original file
/// with the temporary one, minimizing the risk of data corruption.
///
/// # Arguments
///
/// * `file_path_str`: The path to the PDF file to be updated.
/// * `metadata_key`: The key of the metadata entry to set.
/// * `metadata_value`: The value for the metadata entry.
///
/// # Returns
///
/// * `Ok(())` if the update was successful.
/// * `Err(Box<dyn Error>)` if any error occurs during loading, modification,
///   saving to the temporary file, or replacing the original file.
///
/// # Behavior
///
/// * Similar to `set_metadata`, if the `metadata_key` exists, it's overwritten.
/// * An Info dictionary is created if one doesn't exist.
/// * The `ModDate` field is updated.
///
/// # Example
///
/// ```no_run
/// use pdf_metadata::update_metadata_in_place;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let pdf_to_update = "path/to/document_to_update.pdf";
///     let key = "Keywords";
///     let value = "Rust, PDF, Metadata, In-place";
///
///     match update_metadata_in_place(pdf_to_update, key, value) {
///         Ok(_) => println!("Successfully updated metadata in {}", pdf_to_update),
///         Err(e) => eprintln!("Error updating metadata in-place: {}", e),
///     }
///     Ok(())
/// }
/// ```
pub fn update_metadata_in_place(
    file_path_str: &str,
    metadata_key: &str,
    metadata_value: &str,
) -> Result<(), Box<dyn Error>> {
    let original_path = Path::new(file_path_str);

    // Ensure the original file exists before proceeding
    if !original_path.exists() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Original file not found: {}", file_path_str),
        )));
    }

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

    // Create a unique temporary file name in the same directory as the original
    let parent_dir = original_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Failed to determine parent directory for temporary file.")
    })?;
    let original_filename_stem = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("temp_pdf_update"); // Fallback stem
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_micros();
    let temp_filename_str = format!("{}_{}.pdf.tmp", original_filename_stem, timestamp);
    let temp_file_path = parent_dir.join(&temp_filename_str);

    // Save to the temporary file
    if let Err(save_err) = doc.save(&temp_file_path) {
        // Attempt to clean up the temporary file if saving fails
        let _ = fs::remove_file(&temp_file_path);
        return Err(format!("Error saving to temporary file '{}': {}", temp_file_path.display(), save_err).into());
    }

    // Replace the original file with the temporary file
    if let Err(rename_err) = fs::rename(&temp_file_path, original_path) {
        // Attempt to clean up the temporary file if renaming fails
        let _ = fs::remove_file(&temp_file_path);
        return Err(format!("Error renaming temporary file '{}' to original '{}': {}", temp_file_path.display(), original_path.display(), rename_err).into());
    }

    Ok(())
}

/// Retrieves all metadata entries from the Info dictionary of the specified PDF file.
///
/// # Arguments
///
/// * `file_path`: The path to the PDF file from which to read metadata.
///
/// # Returns
///
/// * `Ok(Vec<(String, String)>)`: A vector of tuples, where each tuple contains a
///   metadata key and its corresponding value, both as `String`. If the PDF has no
///   Info dictionary or it's empty, an empty vector is returned.
/// * `Err(Box<dyn Error>)`: An error if the file cannot be loaded, is not a valid PDF,
///   or another I/O error occurs.
///
/// # Example
///
/// ```no_run
/// use pdf_metadata::get_metadata;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     match get_metadata("path/to/document.pdf") {
///         Ok(metadata_list) => {
///             for (key, value) in metadata_list {
///                 println!("Key: {}, Value: {}", key, value);
///             }
///         }
///         Err(e) => eprintln!("Failed to get metadata: {}", e),
///     }
///     Ok(())
/// }
/// ```
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
        if let Ok(info_object_ref) = doc.get_object(info_dict_id) { // Attempt to get the object
            if let Ok(dictionary) = info_object_ref.as_dict() { // Attempt to interpret as dictionary
                for (key_bytes, value_object) in dictionary.iter() {
                    let key = String::from_utf8_lossy(key_bytes).into_owned();
                    let value = info_value_to_string(value_object);
                    metadata_entries.push((key, value));
                }
            }
            // If info_object_ref is not a dictionary, metadata_entries remains empty for this path, which is fine.
        }
        // If info_object_ref cannot be retrieved, metadata_entries remains empty for this path.
    }
    // If info_dict_id_res is Err, it means no Info dictionary reference was found in the trailer.
    // In this case, an empty vector is correctly returned.
    Ok(metadata_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Helper function to create a unique test directory.
    // Returns the path to the created directory.
    fn setup_unique_test_dir(test_name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros();
        let mut temp_dir = env::temp_dir();
        // Create a subdirectory specific to the test run and test name
        temp_dir.push("pdf_metadata_tests");
        temp_dir.push(format!("{}_{}", test_name, millis));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp test directory");
        temp_dir
    }

    // Helper function to create a minimal PDF for testing.
    fn create_minimal_test_pdf(path: &Path) -> Result<(), Box<dyn Error>> {
        let mut doc = Document::with_version("1.7");
        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Count", Object::Integer(0)); // Minimal page count
        pages_dict.set("Kids", Object::Array(vec![])); // No actual pages
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
        let value = "MySetValue Āččęñtš"; // Test with non-ASCII characters
        set_metadata(original_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value)?;

        assert!(output_file.exists(), "Output file should have been created");

        let metadata = get_metadata(output_file.to_str().unwrap())?;
        let entry = metadata.iter().find(|(k, _)| k == key);
        assert!(entry.is_some(), "Metadata key was not found");
        assert_eq!(entry.unwrap().1, value, "Metadata value does not match");

        let mod_date_exists = metadata.iter().any(|(k, _)| k == "ModDate");
        assert!(mod_date_exists, "ModDate should exist");

        fs::remove_dir_all(test_dir)?; // Cleanup
        Ok(())
    }

    #[test]
    fn test_set_metadata_overwrites_existing_key() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("set_metadata_overwrite");
        let original_file = test_dir.join("original_overwrite.pdf");
        let output_file = test_dir.join("output_overwrite.pdf"); // Will be written to twice

        create_minimal_test_pdf(&original_file)?;

        let key = "MyKeyToOverwrite";
        let value1 = "InitialValue";
        let value2 = "OverwrittenValue";

        // First set
        set_metadata(original_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value1)?;
        // Second set, using the output of the first as input, to the same output path
        set_metadata(output_file.to_str().unwrap(), output_file.to_str().unwrap(), key, value2)?;

        let metadata = get_metadata(output_file.to_str().unwrap())?;
        let entry = metadata.iter().find(|(k, _)| k == key);
        assert_eq!(entry.unwrap().1, value2, "Value should have been overwritten");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_metadata_from_pdf_with_no_info_dict() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("get_metadata_no_info");
        let pdf_file = test_dir.join("no_info.pdf");
        // Create a PDF that explicitly does not have an Info dictionary in the trailer
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
        // Intentionally do not set doc.trailer.set("Info", ...);
        doc.save(&pdf_file)?;


        let metadata = get_metadata(pdf_file.to_str().unwrap())?;
        assert!(metadata.is_empty(), "Should return an empty vector for PDF without an Info dictionary");

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
        assert!(entry.is_some(), "Metadata key was not found after in-place update");
        assert_eq!(entry.unwrap().1, value, "Metadata value does not match after in-place update");

        let mod_date_exists = metadata.iter().any(|(k, _)| k == "ModDate");
        assert!(mod_date_exists, "ModDate should exist after in-place update");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }

    #[test]
    fn test_set_metadata_file_not_found_error() {
        let test_dir = setup_unique_test_dir("set_metadata_err_fnf");
        let output_file = test_dir.join("output_err.pdf");
        let result = set_metadata("non_existent_input.pdf", output_file.to_str().unwrap(), "key", "value");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No such file or directory") || e.to_string().contains("entity not found"));
        }
        fs::remove_dir_all(&test_dir).unwrap_or_else(|_| eprintln!("Warning: could not remove test dir {}", test_dir.display()));
    }

    #[test]
    fn test_update_metadata_in_place_file_not_found_error() {
        let result = update_metadata_in_place("non_existent_update.pdf", "key", "value");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Original file not found"));
        }
    }

    #[test]
    fn test_get_metadata_file_not_found_error() {
        let result = get_metadata("non_existent_get.pdf");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No such file or directory") || e.to_string().contains("entity not found"));
        }
    }

    #[test]
    fn test_update_metadata_creates_info_if_not_present() -> Result<(), Box<dyn Error>> {
        let test_dir = setup_unique_test_dir("update_creates_info");
        let pdf_file = test_dir.join("update_creates_info.pdf");

        // Create a PDF that explicitly does not have an Info dictionary in the trailer
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
        doc.save(&pdf_file)?;

        // Check it's empty first
        let initial_metadata = get_metadata(pdf_file.to_str().unwrap())?;
        assert!(initial_metadata.is_empty(), "Initially, metadata should be empty");

        // Now update, which should create the Info dictionary
        let key = "NewlyCreatedKey";
        let value = "This was created";
        update_metadata_in_place(pdf_file.to_str().unwrap(), key, value)?;

        let updated_metadata = get_metadata(pdf_file.to_str().unwrap())?;
        assert!(!updated_metadata.is_empty(), "Metadata should not be empty after update");
        let entry = updated_metadata.iter().find(|(k, _)| k == key);
        assert!(entry.is_some(), "The new key should exist");
        assert_eq!(entry.unwrap().1, value, "The new value should match");

        fs::remove_dir_all(test_dir)?;
        Ok(())
    }
}
