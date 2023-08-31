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
    rust_abf = "0.2.0"
    ```

3. In your Rust code:

    ```rust
    use rust_abf::AbfBuilder;

    fn main() {
        let abf = AbfBuilder::from_file("path/to/your/file.abf");
        match abf {
            Ok(abf) => {
                let channels_count = abf.get_channels_count();
                let sweeps_count = abf.get_sweeps_count();
                println!("There are {:?} channels and {:?} sweeps", channels_count, sweeps_count);
                (0..channels_count).for_each(|ch| {
                    (0..sweeps_count).for_each(|s|{
                        let data = abf.get_sweep_in_channel( s, ch).unwrap();
                        println!("First 10 elements from ch {:?} and sweep {:?}: {:?}", ch, s, &data[0..10]);
                    });
                });
            },
            _ => println!("File not found"),
        }
    }
    ```

If you prefer to work on channels, you can have direct access to them by using the following code:
```rust
    ...
    let ch0 = abf.get_channel(0).unwrap();
    println!("Channel 0 has the following unit of measurement {:?} and the following label {:?}", ch0.get_uom(), ch0.get_label());
    for s in 0..abf.get_sweeps_count() {
        let data = ch0.get_sweep(s).unwrap();
        println!("Sweep {:?} has {:?} points", s, data.len());
    }
    ...
``` 
## Contributing

Contributions are welcome! If you encounter issues or have suggestions, please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.