use colored::{ColoredString, Colorize};

pub struct LogOptions {
    pub border: Option<char>,
    pub border_bottom: Option<char>,
    pub quiet: bool,
}

/// Logs the given text to stdout (if quiet is False) and
/// to an optional log file. By default, we strip out newlines in order to
/// print our lines correctly, but you can override this functionality if you
/// want to print multi-line output.
pub fn log(text: ColoredString, options: LogOptions) {
    if options.quiet {
        return; // Don't log anything if quiet mode is enabled.
    }

    let mut border_string = ColoredString::from("");
    
    // Handle border
    if let Some(border) = options.border {
        let border_length = 79; // Consider making this configurable
        let border_color = text.fgcolor(); // Reuse color for border

        let buffer = std::iter::repeat(border)
            .take(border_length)
            .collect::<String>();
        
        border_string = ColoredString::from(buffer.as_str()); // Convert to &str
        
        if let Some(color) = border_color {
            border_string = border_string.color(color);
        }

        // Print the top border
        println!("{}{}", border_string, ColoredString::from("").clear());
    }

    // Print the text content
    println!("{}{}", text, ColoredString::from("").clear());

    // Handle bottom border if present
    if let Some(_border_bottom) = options.border_bottom {
        // Implement functionality for the bottom border if needed
    } else if !border_string.is_empty() {
        // Print the bottom border (if defined)
        println!("{}{}", border_string, ColoredString::from("").clear());
    }
}
