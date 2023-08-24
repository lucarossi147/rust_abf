# rust_abf

# Rust ABF Reader

This is a Rust project that provides a fast and memory-efficient way to read ABF (Axon Binary Format) files commonly used in electrophysiology.

## Features

- **Fast Reading**: Utilizes optimized algorithms for quick ABF file parsing.
- **Low Memory Footprint**: Minimizes memory usage while processing large ABF files.
- **Minimal Dependencies**: Strives to keep dependencies lightweight.
- **Easy to Use**: Provides a simple API for reading ABF files.

## Usage

1. Install Rust and Cargo if you haven't already.
2. Add this library to your `Cargo.toml`:

    ```toml
    [dependencies]
    rust-abf-reader = "0.1"
    ```

3. In your Rust code:

    ```rust
    use rust_abf;

    fn main() {
        let file_path = "path/to/your/file.abf";
        
        if let Ok(reader) = AbfReader::new(file_path) {
            // Read header information
            let header = reader.header();

            // Access channels and data
            for channel in header.channel_info.iter() {
                let channel_number = channel.channel_number;
                let data = reader.read_channel_data(channel_number);
                // Process data as needed
            }
        } else {
            eprintln!("Error reading ABF file");
        }
    }
    ```

## Contributing

Contributions are welcome! If you encounter issues or have suggestions, please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.