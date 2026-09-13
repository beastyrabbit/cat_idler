using System;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using IdleCatForest.Acceptance;

class Program
{
    static int Main(string[] args)
    {
        string filter = args.FirstOrDefault(a => a.StartsWith("--filter="))?.Substring(9) ?? "";
        var cases = AcceptanceScenarios.Cases().Where(c => c.Name.Contains(filter, StringComparison.OrdinalIgnoreCase) && (!args.Contains("--exclude-campaigns") || !c.Name.StartsWith("campaign.", StringComparison.Ordinal))).ToArray();
        if (args.Contains("--list")) { foreach (var c in cases) Console.WriteLine(c.Name); return 0; }
        if (cases.Length == 0) { Console.Error.WriteLine("No matching scenarios"); return 2; }
        int failed = 0; var all = Stopwatch.StartNew();
        foreach (var c in cases) { var clock = Stopwatch.StartNew(); try { c.Run(); Console.WriteLine("PASS " + c.Name + " " + clock.Elapsed.TotalMilliseconds.ToString("F1", CultureInfo.InvariantCulture) + "ms"); } catch (Exception e) { failed++; Console.WriteLine("FAIL " + c.Name + " " + e.Message); } }
        Console.WriteLine("RESULT total=" + cases.Length + " failed=" + failed + " elapsed_ms=" + all.Elapsed.TotalMilliseconds.ToString("F1", CultureInfo.InvariantCulture)); return failed == 0 ? 0 : 1;
    }
}
