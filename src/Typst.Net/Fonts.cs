namespace Typst;

public record Fonts(
    bool IncludeSystemFonts = true,
    IEnumerable<string>? FontPaths = null
);
