using System.IO;
using System.Text;
using System.Collections.Generic;

namespace Typst.Net.Tests;

public class CompileTests
{
    [Fact]
    public void TestCompileSimpleDocumentToPdf()
    {
        using var compiler = new TypstCompiler("= Hello, World!");
        var (pages, warnings) = compiler.Compile();
        var pdfBytes = pages.First();

        Assert.NotNull(pdfBytes);
        Assert.NotEmpty(pdfBytes);
        Assert.Empty(warnings);

        var pdfHeader = Encoding.ASCII.GetString(pdfBytes, 0, 5);
        Assert.Equal("%PDF-", pdfHeader);
    }

    [Fact]
    public void TestCompileToPngWithPpi()
    {
        using var compiler = new TypstCompiler("Hello, PNG!");
        var (pages, _) = compiler.Compile(format: "png", ppi: 200.0f);
        var pngBytes = pages.First();
        Assert.NotNull(pngBytes);
        Assert.NotEmpty(pngBytes);
        Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, pngBytes.Take(8));
    }

    [Fact]
    public void TestCompileToSvg()
    {
        using var compiler = new TypstCompiler("Hello, SVG!");
        var (pages, _) = compiler.Compile(format: "svg");
        var svgBytes = pages.First();
        var svgText = Encoding.UTF8.GetString(svgBytes);
        Assert.StartsWith("<svg", svgText);
    }

    [Fact]
    public void TestCompileToFile()
    {
        var outputFile = "output.pdf";
        using var compiler = new TypstCompiler("Hello, File!");
        compiler.Compile(outputFile, "pdf");

        Assert.True(File.Exists(outputFile));
        var pdfBytes = File.ReadAllBytes(outputFile);
        Assert.NotEmpty(pdfBytes);
        File.Delete(outputFile);
    }

    [Fact]
    public void TestCompileWithInvalidSyntax()
    {
        using var compiler = new TypstCompiler("#let a = (1");
        var exception = Assert.Throws<Exception>(() => compiler.Compile());
        Assert.Contains("unclosed delimiter", exception.Message);
    }

    
}