# Typst.Net

> **Acknowledgement**
> 
> This project's implementation is heavily inspired by the Python binding concepts in [messense/typst-py](https://github.com/messense/typst-py). Special thanks to the original author for their contributions.

> **⚠️ Warning**
> 
> This codebase was primarily generated with the assistance of the Gemini CLI and is a product of AI Vibe-Driven Development. Although it has undergone multiple rounds of testing, it may still contain potential errors or deviate from best practices. Before using in a production environment, please be sure to carefully review the code and test it thoroughly.

A .NET wrapper for the [Typst](https://github.com/typst/typst) typesetting system, allowing you to compile Typst code from your C# applications.

## Solution Structure

- **/src/Typst.Net**: The main C# class library project.
- **/src/rust_core**: The Rust FFI project that wraps the core Typst logic.
- **/tests/Typst.Net.Tests**: The xUnit test project for the C# wrapper.

## How to Build

1.  **Install Rust:** Make sure you have the Rust toolchain installed.
2.  **Install .NET:** Make sure you have the .NET SDK installed.
3.  **Build the solution:** Run `dotnet build` in this directory. The C# project is configured to automatically build the Rust FFI library first.

## How to Use

### Basic Example

```csharp
using Typst;

// Create a compiler instance with your Typst source code.
using var compiler = new TypstCompiler("= Hello, C#!\n");

// Compile to a PDF.
var (pages, warnings) = compiler.Compile();
File.WriteAllBytes("output.pdf", pages.First());

// Query the document.
string queryResult = compiler.Query("heading");
Console.WriteLine(queryResult);
```

### Advanced Example

```csharp
using Typst;

// Configure fonts and system inputs
var fonts = new Fonts(
    IncludeSystemFonts: true,
    FontPaths: new[] { "C:/Windows/Fonts" } // Optional extra font paths
);

var sysInputs = new Dictionary<string, string> { { "user", "Gemini" } };

var source = """
#set text(font: \"Arial\")

= Hello, #sys.inputs.user!

This is a test.
""";

// Create a compiler with advanced options
using var compiler = new TypstCompiler(source, fonts: fonts, sysInputs: sysInputs);

// Compile to a multi-page PNG
var (pages, warnings) = compiler.Compile(format: "png", ppi: 150.0f);

// Save the first page
if (pages.Any()) {
    File.WriteAllBytes("output.png", pages.First());
}

// Handle any warnings
foreach (var warning in warnings) 
{
    Console.WriteLine($"Warning: {warning.Message}");
}

// Perform a specific query
string? firstHeading = compiler.Query("heading", one: true);
Console.WriteLine($"First heading: {firstHeading}");
```

## Packaging & Publishing

### 1. Create the NuGet Package

To package the project into a `.nupkg` file, run the `dotnet pack` command from the `csharp` directory. Be sure to build in `Release` configuration.

```powershell
dotnet pack ./src/Typst.Net/Typst.Net.csproj -c Release
```

This will create the package in the `src/Typst.Net/bin/Release` directory.

### 2. Publish the Package

Once you have the `.nupkg` file, you can publish it to a NuGet feed like [nuget.org](https://www.nuget.org/). You will need an API key from the feed provider.

```powershell
# Replace with the actual path to your package and your API key
dotnet nuget push ./src/Typst.Net/bin/Release/Typst.Net.0.1.0.nupkg --api-key YOUR_API_KEY --source https://api.nuget.org/v3/index.json
```