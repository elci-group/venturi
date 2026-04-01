#[cfg(test)]
mod tests {
    use venturi::lexer::Lexer;
    use venturi::parser::Parser;
    
    // Testing Venturi's ability to provide actionable errors for LLMs
    #[test]
    fn test_actionable_error_messages() {
        let code = r#"
#!plane
# Intentionally incorrect arrow syntax
input data: DataFrame
data => card
"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_err());
        let err_msg = tokens.unwrap_err().to_string();
        
        // Output the error message for debugging
        println!("Error: {}", err_msg);
        
        // Assert that the error message contains the specific location and reasoning
        assert!(err_msg.contains("Parse error"));
        assert!(err_msg.contains("Unexpected character: '>'"));
    }

    #[test]
    fn test_zero_shot_llm_code() {
        let code = r#"
#!plane
# LLM generated code for a basic transform
input raw_data: DataFrame
output clean_data: DataFrame

func transform():
    return clean(raw_data)

raw_data -> transform
transform -> clean_data
"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse_file();
        
        assert!(result.is_ok());
    }
}
