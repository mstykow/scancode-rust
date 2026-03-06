fn main() {
    let chars = vec![
        ('\u{00A9}', "copyright"),
        ('\u{201C}', "left double quote"),
        ('\u{201D}', "right double quote"),
        ('\u{2212}', "minus sign"),
        ('\u{2019}', "right single quote"),
        ('\u{00DF}', "German sharp s"),
        ('\u{00E4}', "German a-umlaut"),
    ];
    
    println!("Deunicode transliteration:");
    for (c, name) in chars {
        let result = deunicode::deunicode_char(c);
        println!("  {} (U+{:04X}, {}) -> {:?}", c, c as u32, name, result);
    }
    
    println!("\nFull text example:");
    let text = "European Union Public Licence \u{201C}EUPL\u{201D} \u{00A9} the European Community 2007";
    let result = deunicode::deunicode(text);
    println!("  Original: {}", text);
    println!("  Result:   {}", result);
    println!("  Difference: Copyright symbol became (c), quotes became regular quotes");
}
