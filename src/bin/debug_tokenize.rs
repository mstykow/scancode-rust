use scancode_rust::license_detection::tokenize::tokenize_with_stopwords;

fn main() {
    let rule_text = "\navailable for use under the following license, commonly known\nas the 3-clause (or \"{{modified\") BSD license}}:";
    
    println!("Rule text: {:?}", rule_text);
    
    let (tokens, stopwords) = tokenize_with_stopwords(rule_text);
    
    println!("Tokens ({}): {:?}", tokens.len(), tokens);
    println!("Stopwords: {:?}", stopwords);
    
    // Test the test file
    let test_text = "Libevent is available for use under the following license, commonly known
as the 3-clause (or \"modified\") BSD license:";
    
    println!("\nTest text: {:?}", test_text);
    let (test_tokens, test_stopwords) = tokenize_with_stopwords(test_text);
    println!("Test tokens ({}): {:?}", test_tokens.len(), test_tokens);
}
