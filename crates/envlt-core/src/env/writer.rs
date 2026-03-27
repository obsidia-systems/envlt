use std::collections::BTreeMap;

pub fn render_env(variables: &BTreeMap<String, String>) -> String {
    let mut output = String::new();

    for (key, value) in variables {
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }

    output
}
