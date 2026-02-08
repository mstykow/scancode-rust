use scancode_rust::parsers::*;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <parser_type> <input_file> <output_file>",
            args[0]
        );
        eprintln!(
            "Example: {} deb testdata/debian/deb/adduser.deb testdata/debian/deb/adduser.deb.expected.json",
            args[0]
        );
        std::process::exit(1);
    }

    let parser_type = &args[1];
    let input_file = PathBuf::from(&args[2]);
    let output_file = PathBuf::from(&args[3]);

    let package_data = match parser_type.as_str() {
        "deb" => DebianDebParser::extract_package_data(&input_file),
        "dsc" => DebianDscParser::extract_package_data(&input_file),
        "debian-control" => DebianControlParser::extract_package_data(&input_file),
        "debian-installed" => DebianInstalledParser::extract_package_data(&input_file),
        "debian-copyright" => DebianCopyrightParser::extract_package_data(&input_file),
        "alpine-installed" => AlpineInstalledParser::extract_package_data(&input_file),
        "alpine-apk" => AlpineApkParser::extract_package_data(&input_file),
        "rpm" => RpmParser::extract_package_data(&input_file),
        _ => {
            eprintln!("Unknown parser type: {}", parser_type);
            std::process::exit(1);
        }
    };

    let json = serde_json::to_string_pretty(&vec![package_data]).unwrap();
    fs::write(&output_file, json).unwrap();
    println!("Generated: {}", output_file.display());
}
