//! PDF text extraction module
//!
//! Extracts text content from PDF files using lopdf.

use crate::errors::IngestionError;
use std::path::Path;
use tracing::{debug, warn};

/// Extract text content from a PDF file
pub fn extract_text_from_pdf(path: &Path) -> Result<String, IngestionError> {
    let doc = lopdf::Document::load(path).map_err(|e| IngestionError::PdfParseError {
        path: path.display().to_string(),
        message: format!("Failed to load PDF: {}", e),
    })?;

    let mut text = String::new();
    let pages = doc.get_pages();
    
    debug!(page_count = pages.len(), "Extracting text from PDF");

    for (page_num, _) in pages.iter() {
        match extract_page_text(&doc, *page_num) {
            Ok(page_text) => {
                text.push_str(&page_text);
                text.push('\n');
            }
            Err(e) => {
                warn!(page = page_num, error = %e, "Failed to extract text from page, skipping");
            }
        }
    }

    if text.trim().is_empty() {
        return Err(IngestionError::PdfParseError {
            path: path.display().to_string(),
            message: "No text content extracted from PDF".to_string(),
        });
    }

    // Clean up the extracted text
    let cleaned = clean_text(&text);
    
    debug!(
        original_len = text.len(),
        cleaned_len = cleaned.len(),
        "Text extraction complete"
    );

    Ok(cleaned)
}

/// Extract text from a single page
fn extract_page_text(doc: &lopdf::Document, page_num: u32) -> Result<String, String> {
    let page_id = doc
        .page_iter()
        .nth((page_num - 1) as usize)
        .ok_or_else(|| format!("Page {} not found", page_num))?;

    let content = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
    
    // Parse content stream and extract text
    let text = extract_text_from_content(&content);
    
    Ok(text)
}

/// Extract text from PDF content stream
fn extract_text_from_content(content: &[u8]) -> String {
    // Simple text extraction - looks for text between BT and ET operators
    let content_str = String::from_utf8_lossy(content);
    let mut text = String::new();
    let mut in_text_block = false;
    let mut current_text = String::new();

    for line in content_str.lines() {
        let trimmed = line.trim();
        
        if trimmed == "BT" {
            in_text_block = true;
            continue;
        }
        
        if trimmed == "ET" {
            in_text_block = false;
            if !current_text.is_empty() {
                text.push_str(&current_text);
                text.push(' ');
                current_text.clear();
            }
            continue;
        }
        
        if in_text_block {
            // Look for text showing operators: Tj, TJ, ', "
            if let Some(text_content) = extract_text_from_operator(trimmed) {
                current_text.push_str(&text_content);
            }
        }
    }

    text
}

/// Extract text from a PDF text operator
fn extract_text_from_operator(line: &str) -> Option<String> {
    // Handle (text) Tj operator
    if line.ends_with("Tj") || line.ends_with("'") || line.ends_with("\"") {
        if let Some(start) = line.find('(') {
            if let Some(end) = line.rfind(')') {
                let text = &line[start + 1..end];
                return Some(decode_pdf_string(text));
            }
        }
    }
    
    // Handle [(text) num (text) num] TJ operator (array of text)
    if line.ends_with("TJ") {
        let mut result = String::new();
        let mut in_paren = false;
        let mut current = String::new();
        
        for ch in line.chars() {
            match ch {
                '(' => {
                    in_paren = true;
                }
                ')' => {
                    in_paren = false;
                    result.push_str(&decode_pdf_string(&current));
                    current.clear();
                }
                _ if in_paren => {
                    current.push(ch);
                }
                _ => {}
            }
        }
        
        if !result.is_empty() {
            return Some(result);
        }
    }
    
    None
}

/// Decode PDF string escapes
fn decode_pdf_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('(') => result.push('('),
                Some(')') => result.push(')'),
                Some(c) => result.push(c),
                None => {}
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Clean extracted text
fn clean_text(text: &str) -> String {
    text
        // Replace multiple whitespace with single space
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        // Remove common PDF artifacts
        .replace("", "") // Remove BOM
        .replace("\u{FEFF}", "")
        // Normalize quotes
        .replace('"', "\"")
        .replace('"', "\"")
        .replace(''', "'")
        .replace(''', "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let input = "Hello   World\n\nTest";
        let cleaned = clean_text(input);
        assert_eq!(cleaned, "Hello World Test");
    }

    #[test]
    fn test_decode_pdf_string() {
        assert_eq!(decode_pdf_string("Hello\\nWorld"), "Hello\nWorld");
        assert_eq!(decode_pdf_string("Test\\(paren\\)"), "Test(paren)");
    }
}
