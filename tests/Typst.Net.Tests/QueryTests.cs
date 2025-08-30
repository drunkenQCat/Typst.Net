using System.IO;
using System.Text.Json;

namespace Typst.Net.Tests;

public class QueryTests
{
    [Fact]
    public void TestQueryHeadingsSimple()
    {
        using var compiler = new TypstCompiler("= Heading 1\n== Heading 2");
        var queryResult = compiler.Query("heading");

        Assert.NotNull(queryResult);
        Assert.NotEmpty(queryResult);

        var json = JsonDocument.Parse(queryResult);
        Assert.Equal(JsonValueKind.Array, json.RootElement.ValueKind);
        Assert.Equal(2, json.RootElement.GetArrayLength());
    }

    [Fact]
    public void TestQueryFootnotesComplex()
    {
        var input = File.ReadAllText("hello.typ");
        using var compiler = new TypstCompiler(input);
        var queryResult = compiler.Query("<footnote-1>");

        Assert.NotNull(queryResult);
        Assert.NotEmpty(queryResult);

        var json = JsonDocument.Parse(queryResult);
        Assert.Equal(JsonValueKind.Array, json.RootElement.ValueKind);
        Assert.True(json.RootElement.GetArrayLength() > 0);
    }

    [Fact]
    public void TestQueryWithField()
    {
        var input = File.ReadAllText("hello.typ");
        using var compiler = new TypstCompiler(input);
        var queryResult = compiler.Query("heading", field: "body");

        Assert.NotNull(queryResult);
        Assert.StartsWith("[", queryResult);
        Assert.EndsWith("]", queryResult);
        Assert.Contains("Typst", queryResult);
    }

    [Fact]
    public void TestQueryWithOne()
    {
        var input = File.ReadAllText("hello.typ");
        using var compiler = new TypstCompiler(input);
        var queryResult = compiler.Query("heading.where(level: 1)", one: true);

        Assert.NotNull(queryResult);
        Assert.StartsWith("{", queryResult); // Should be a single JSON object, not an array
        Assert.EndsWith("}", queryResult);
    }
}