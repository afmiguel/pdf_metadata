# PDF Metadata Utility

A Rust library for reading, setting, and updating metadata in PDF files. This library allows you to interact with the PDF's "Info" dictionary, where standard metadata fields like Author, Title, Subject, Keywords, Creator, Producer, CreationDate, and ModDate are typically stored, as well as custom metadata entries.

## Features

* **Get Metadata**: Retrieve all entries from a PDF's Info dictionary.
* **Set Metadata**: Add or update a specific metadata key-value pair and save the changes to a new PDF file. Automatically updates the `ModDate` field.
* **Update Metadata In-Place**: Add or update a specific metadata key-value pair in an existing PDF file safely (by writing to a temporary file first). Automatically updates the `ModDate` field.

## Adding to Your Project

To use this library in your Rust project, add the following to your `Cargo.toml` file:

```toml
[dependencies]
pdf_metadata = { git = "[https://github.com/afmiguel/pdf_metadata.git](https://github.com/afmiguel/pdf_metadata.git)", branch = "master" } # Or specify a tag/commit for stability
```

Make sure to replace `branch = "master"` with a specific tag (e.g., `tag = "v0.1.0"`) or commit hash (`rev = "commit_hash"`) once you have stable releases, for better dependency management.

## Usage

### Public Functions

#### 1. `get_metadata(file_path: &str) -> Result<Vec<(String, String)>, Box<dyn Error>>`

Retrieves all metadata entries from the Info dictionary of the specified PDF file.

* **Parameters**:
    * `file_path: &str`: The path to the PDF file from which to read metadata.
* **Returns**:
    * `Ok(Vec<(String, String)>)`: A vector of tuples, where each tuple contains a metadata key and its corresponding value, both as `String`. If the PDF has no Info dictionary or it's empty, an empty vector is returned.
    * `Err(Box<dyn Error>)`: An error if the file cannot be loaded, is not a valid PDF, or another I/O error occurs.
* **Example**:

    ```rust
    use pdf_metadata::get_metadata;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        match get_metadata("path/to/your/document.pdf") {
            Ok(metadata_list) => {
                if metadata_list.is_empty() {
                    println!("No metadata found or Info dictionary is empty.");
                } else {
                    println!("PDF Metadata:");
                    for (key, value) in metadata_list {
                        println!("  {}: {}", key, value);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error getting metadata: {}", e);
            }
        }
        Ok(())
    }
    ```

#### 2. `set_metadata(file_path: &str, output_path: &str, metadata_key: &str, metadata_value: &str) -> Result<(), Box<dyn Error>>`

Loads a PDF from `file_path`, sets (adds or updates) a specific metadata entry in its Info dictionary, updates the `ModDate` field to the current time, and saves the modified PDF to `output_path`.

* **Parameters**:
    * `file_path: &str`: The path to the original PDF file.
    * `output_path: &str`: The path where the modified PDF file will be saved. This can be the same as `file_path` if you intend to overwrite, but for safety, `update_metadata_in_place` is generally preferred for in-place modifications.
    * `metadata_key: &str`: The key of the metadata entry to set (e.g., "Author", "MyCustomKey").
    * `metadata_value: &str`: The value for the metadata entry.
* **Returns**:
    * `Ok(())`: If the operation was successful.
    * `Err(Box<dyn Error>)`: If any error occurs during loading, modification, or saving.
* **Behavior**:
    * If the `metadata_key` already exists, its value will be overwritten.
    * If the PDF does not have an Info dictionary, one will be created.
    * The `ModDate` field in the Info dictionary will be set to the current system time.
* **Example**:

    ```rust
    use pdf_metadata::set_metadata;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let original_pdf = "path/to/input.pdf";
        let modified_pdf = "path/to/output_with_metadata.pdf";
        let key = "Author";
        let value = "Jane Doe";

        match set_metadata(original_pdf, modified_pdf, key, value) {
            Ok(_) => println!("Successfully set metadata and saved to {}", modified_pdf),
            Err(e) => eprintln!("Error setting metadata: {}", e),
        }
        Ok(())
    }
    ```

#### 3. `update_metadata_in_place(file_path_str: &str, metadata_key: &str, metadata_value: &str) -> Result<(), Box<dyn Error>>`

Updates (adds or overwrites) a specific metadata entry in the Info dictionary of the specified PDF file and saves the changes back to the same file. This operation is performed safely by first saving to a temporary file and then replacing the original. The `ModDate` field is also updated.

* **Parameters**:
    * `file_path_str: &str`: The path to the PDF file to be updated.
    * `metadata_key: &str`: The key of the metadata entry to set.
    * `metadata_value: &str`: The value for the metadata entry.
* **Returns**:
    * `Ok(())`: If the update was successful.
    * `Err(Box<dyn Error>)`: If any error occurs during loading, modification, saving to the temporary file, or replacing the original file.
* **Behavior**:
    * Similar to `set_metadata`, if the `metadata_key` exists, it's overwritten.
    * An Info dictionary is created if one doesn't exist.
    * The `ModDate` field is updated.
    * The update is performed by writing to a temporary file first, then renaming it to the original file path to minimize risk of data corruption.
* **Example**:

    ```rust
    use pdf_metadata::update_metadata_in_place;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let pdf_to_update = "path/to/document_to_update.pdf";
        let key = "Keywords";
        let value = "Rust, PDF, Metadata";

        match update_metadata_in_place(pdf_to_update, key, value) {
            Ok(_) => println!("Successfully updated metadata in {}", pdf_to_update),
            Err(e) => eprintln!("Error updating metadata in-place: {}", e),
        }
        Ok(())
    }
    ```

### Notes

* **Character Encoding**: PDF string objects can have complex encoding. This library uses `lopdf`'s `Object::string_literal` for writing, which handles encoding to PDFDocEncoding or UTF-16BE. When reading, it attempts to decode strings using `Object::as_str()` and falls back to a lossy UTF-8 conversion if that fails or if the internal representation is raw bytes.
* **`ModDate`**: Both `set_metadata` and `update_metadata_in_place` automatically update the `ModDate` field in the PDF's Info dictionary to reflect the time of modification. The format is a PDF Date string (e.g., `D:YYYYMMDDHHmmSSOHH'mm'`).

## Contributing

Feel free to open issues or submit pull requests on the [GitHub repository](https://github.com/afmiguel/pdf_metadata.git).

## License

This library is licensed under [LICENSE_TYPE] (e.g., MIT or Apache-2.0 - *you'll need to add a license file to your repository and specify it here*).
