use regex::Regex;

#[test]
fn test_tokenization() {
    let pattern = Regex::new(r"[^_\W]+\+?[^_\W]*").unwrap();
    
    let rule_text = r#"License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law."#;
    
    let query_text = r#"License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law."#;
    
    let rule_tokens: Vec<_> = pattern.find_iter(&rule_text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect();
    
    let query_tokens: Vec<_> = pattern.find_iter(&query_text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect();
    
    println!("Rule tokens ({}): {:?}", rule_tokens.len(), rule_tokens);
    println!("\nQuery tokens ({}): {:?}", query_tokens.len(), query_tokens);
    
    assert_eq!(rule_tokens, query_tokens, "Rule and query tokens should match");
}
