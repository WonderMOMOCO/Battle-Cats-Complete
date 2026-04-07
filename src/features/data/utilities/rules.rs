use regex::{Regex, RegexSet};
use crate::global::io::patterns;
use crate::features::settings::logic::exceptions::{ExceptionList, ExceptionRule};

pub fn compile() -> (RegexSet, Vec<ExceptionRule>) {
    let exceptions = ExceptionList::load_or_default();
    
    let mut patterns_for_set = Vec::new();
    let mut active_rules = Vec::new();
    
    let lang_codes: Vec<&str> = patterns::APP_LANGUAGES.iter().map(|&(code, _)| code).collect();
    let lang_string = format!(r"(?:_(?:{}))?", lang_codes.join("|"));
    
    for rule in exceptions.rules {
        if rule.pattern.is_empty() && rule.extension.is_empty() { continue; }
        let extension_string = if rule.extension.is_empty() { String::new() } else { format!(r"\.(?:{})", rule.extension) };
        let pattern = format!(r"^(?:{}){}{}$", rule.pattern, lang_string, extension_string);
        
        if Regex::new(&pattern).is_ok() {
            patterns_for_set.push(pattern);
            active_rules.push(rule);
        }
    }
    
    let regex_set = RegexSet::new(&patterns_for_set).unwrap_or_else(|_| RegexSet::empty());
    (regex_set, active_rules)
}