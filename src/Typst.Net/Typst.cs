using System.Runtime.InteropServices;
using System.Text.Json;

namespace Typst;

public record TypstWarning(string Message);

public unsafe class TypstCompiler : IDisposable
{
    private CsBindgen.Compiler* _compiler;
    private bool _disposed = false;

    public TypstCompiler(string input, Fonts? fonts = null, Dictionary<string, string>? sysInputs = null)
    {
        fonts ??= new Fonts();
        var fontPaths = fonts.FontPaths ?? Enumerable.Empty<string>();
        bool ignoreSystemFonts = !fonts.IncludeSystemFonts;

        var inputPtr = Marshal.StringToHGlobalAnsi(input);

        var fontPathsList = fontPaths.ToList();
        var fontPathPtrs = new IntPtr[fontPathsList.Count];
        for (int i = 0; i < fontPathsList.Count; i++)
        {
            fontPathPtrs[i] = Marshal.StringToHGlobalAnsi(fontPathsList[i]);
        }

        var sysInputsJson = JsonSerializer.Serialize(sysInputs ?? new Dictionary<string, string>());
        var sysInputsPtr = Marshal.StringToHGlobalAnsi(sysInputsJson);

        try
        {
            fixed (IntPtr* fontPathsRawPtr = fontPathPtrs)
            {
                _compiler = CsBindgen.NativeMethods.create_compiler((byte*)inputPtr, (byte**)fontPathsRawPtr, (nuint)fontPathsList.Count, (byte*)sysInputsPtr, ignoreSystemFonts);
            }

            if (_compiler == null)
            {
                throw new Exception("Failed to create Typst compiler.");
            }
        }
        finally
        {
            Marshal.FreeHGlobal(inputPtr);
            foreach (var ptr in fontPathPtrs) Marshal.FreeHGlobal(ptr);
            Marshal.FreeHGlobal(sysInputsPtr);
        }
    }

    public (List<byte[]> pages, List<TypstWarning> warnings) Compile(string format = "pdf", float ppi = 144.0f)
    {
        if (_disposed) throw new ObjectDisposedException(nameof(TypstCompiler));

        var formatPtr = Marshal.StringToHGlobalAnsi(format);
        try
        {
            var result = CsBindgen.NativeMethods.compile(_compiler, (byte*)formatPtr, ppi);
            if (result.error != null)
            {
                var error = Marshal.PtrToStringAnsi((IntPtr)result.error);
                CsBindgen.NativeMethods.free_compile_result(result);
                throw new Exception(error);
            }

            var pages = new List<byte[]>();
            for (int i = 0; i < (int)result.buffers_len; i++)
            {
                var buffer = result.buffers[i];
                var pageBytes = new byte[buffer.len];
                Marshal.Copy((IntPtr)buffer.ptr, pageBytes, 0, (int)buffer.len);
                pages.Add(pageBytes);
            }

            var warnings = new List<TypstWarning>();
            for (int i = 0; i < (int)result.warnings_len; i++)
            {
                var warning = result.warnings[i];
                var message = Marshal.PtrToStringAnsi((IntPtr)warning.message);
                warnings.Add(new TypstWarning(message ?? ""));
            }

            CsBindgen.NativeMethods.free_compile_result(result);
            return (pages, warnings);
        }
        finally
        {
            Marshal.FreeHGlobal(formatPtr);
        }
    }

    public void Compile(string outputFile, string format, float ppi = 144.0f)
    {
        var (pages, _) = Compile(format, ppi);
        if (pages.Count == 1)
        {
            File.WriteAllBytes(outputFile, pages[0]);
        }
        else
        {
            var extension = Path.GetExtension(outputFile);
            var fileName = Path.GetFileNameWithoutExtension(outputFile);
            var directory = Path.GetDirectoryName(outputFile) ?? "";

            for (int i = 0; i < pages.Count; i++)
            {
                var pagePath = Path.Combine(directory, $"{fileName}-{i + 1}{extension}");
                File.WriteAllBytes(pagePath, pages[i]);
            }
        }
    }

    public string Query(string selector, string? field = null, bool one = false)
    {
        if (_disposed) throw new ObjectDisposedException(nameof(TypstCompiler));

        var selectorPtr = Marshal.StringToHGlobalAnsi(selector);
        var fieldPtr = field == null ? IntPtr.Zero : Marshal.StringToHGlobalAnsi(field);
        try
        {
            var resultPtr = CsBindgen.NativeMethods.query(_compiler, (byte*)selectorPtr, (byte*)fieldPtr, one);
            var result = Marshal.PtrToStringAnsi((IntPtr)resultPtr);
            CsBindgen.NativeMethods.free_string(resultPtr);
            return result ?? "";
        }
        finally
        {
            Marshal.FreeHGlobal(selectorPtr);
            if (fieldPtr != IntPtr.Zero) Marshal.FreeHGlobal(fieldPtr);
        }
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            CsBindgen.NativeMethods.free_compiler(_compiler);
            _compiler = null;
            _disposed = true;
            GC.SuppressFinalize(this);
        }
    }

    ~TypstCompiler()
    {
        Dispose();
    }
}