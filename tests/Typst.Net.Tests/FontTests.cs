namespace Typst.Net.Tests;

public class FontTests
{
    [Fact]
    public void TestCompileWithNoSystemFonts()
    {
        // This test ensures that compilation succeeds even when system fonts are disabled.
        // A more robust test would check for font fallback, but that's harder to assert.
        var input = "Hello, world!";
        var fonts = new Fonts(IncludeSystemFonts: false);
        using var compiler = new TypstCompiler(input, fonts: fonts);
        var (pages, warnings) = compiler.Compile();

        Assert.NotEmpty(pages);
        // We might get warnings about font fallbacks, which is expected.
    }

    [Fact]
    public void TestCompileWithFontPaths()
    {
        // This test ensures that compilation succeeds when custom font paths are provided.
        // It doesn't check if the fonts are actually used, just that the parameter is accepted.
        var input = "Hello, world!";
        var fonts = new Fonts(FontPaths: new[] { "/dummy/path" }); // Path doesn't need to exist for this test
        using var compiler = new TypstCompiler(input, fonts: fonts);
        var (pages, warnings) = compiler.Compile();

        Assert.NotEmpty(pages);
    }
}
