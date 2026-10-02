using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Xml;
using IdleCatForest.Acceptance;

/// <summary>
/// Rendering-free scenario runner. Exit codes: 0 all selected cases passed, 1 at least one case failed,
/// 2 invalid arguments, 3 no case matched the selection.
/// </summary>
static class Program
{
    const string Usage =
        "Usage: Forest.Scenarios [--jobs=N] [--filter=TEXT] [--include-campaigns] [--include-demo-timing] [--list] [--junit=PATH]\n" +
        "  --jobs=N               run N cases at once; 0 = processor count (default 1)\n" +
        "  --filter=TEXT          case-insensitive substring of the case name (also --filter TEXT)\n" +
        "  --include-campaigns    also select names starting with campaign.\n" +
        "  --include-demo-timing  also select names starting with demo_timing.\n" +
        "  --exclude-campaigns    accepted for compatibility; campaigns are excluded by default\n" +
        "  --list                 print the selected names and exit\n" +
        "  --junit=PATH           also write a JUnit XML report to PATH";

    sealed class Options
    {
        public int Jobs = 1;
        public string Filter = "";
        public bool IncludeCampaigns, IncludeDemoTiming, List, Help;
        public string JUnitPath = "";
    }

    sealed class Outcome
    {
        public bool Passed;
        public double Milliseconds;
        public string Output = "";
        public Exception Error;
    }

    static int Main(string[] args)
    {
        Options options;
        try { options = Parse(args); }
        catch (ArgumentException e)
        {
            Console.Error.WriteLine("error: " + e.Message);
            Console.Error.WriteLine(Usage);
            return 2;
        }
        if (options.Help) { Console.WriteLine(Usage); return 0; }

        var cases = AcceptanceScenarios.Cases().Where(c => Selected(c.Name, options)).ToArray();
        if (cases.Length == 0)
        {
            Console.Error.WriteLine("selected=0: no scenario matches the selection");
            return 3;
        }
        if (options.List)
        {
            foreach (var c in cases) Console.WriteLine(c.Name);
            return 0;
        }

        int jobs = options.Jobs == 0 ? Environment.ProcessorCount : options.Jobs;
        var outcomes = new Outcome[cases.Length];
        var originalOut = Console.Out;
        var router = new RoutingWriter(originalOut);
        Console.SetOut(router);
        var clock = Stopwatch.StartNew();
        try
        {
            var partitioner = Partitioner.Create(Enumerable.Range(0, cases.Length), EnumerablePartitionerOptions.NoBuffering);
            Parallel.ForEach(partitioner, new ParallelOptions { MaxDegreeOfParallelism = jobs }, i => outcomes[i] = RunCase(cases[i], router));
        }
        finally { Console.SetOut(originalOut); }
        clock.Stop();

        int failed = 0;
        foreach (int i in Enumerable.Range(0, cases.Length).OrderBy(i => cases[i].Name, StringComparer.Ordinal))
        {
            var outcome = outcomes[i];
            string time = outcome.Milliseconds.ToString("F1", CultureInfo.InvariantCulture) + "ms";
            if (outcome.Passed) Console.WriteLine("PASS " + cases[i].Name + " " + time);
            else
            {
                failed++;
                var error = Unwrap(outcome.Error);
                Console.WriteLine("FAIL " + cases[i].Name + " " + time);
                Console.WriteLine("  " + error.GetType().FullName + ": " + error.Message);
                foreach (var frame in TopFrames(error, 5)) Console.WriteLine("    " + frame);
            }
            if (outcome.Output.Length > 0)
                foreach (var line in outcome.Output.TrimEnd().Split('\n')) Console.WriteLine("  | " + line.TrimEnd('\r'));
        }

        long elapsed = (long)Math.Round(clock.Elapsed.TotalMilliseconds);
        if (options.JUnitPath.Length > 0) WriteJUnit(options.JUnitPath, cases, outcomes, clock.Elapsed.TotalSeconds);
        Console.WriteLine("selected=" + cases.Length + " passed=" + (cases.Length - failed) + " failed=" + failed + " elapsed_ms=" + elapsed.ToString(CultureInfo.InvariantCulture));
        return failed == 0 ? 0 : 1;
    }

    static Options Parse(string[] args)
    {
        var options = new Options();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        for (int i = 0; i < args.Length; i++)
        {
            string arg = args[i];
            string name = arg, value = null;
            int equals = arg.IndexOf('=');
            if (arg.StartsWith("--", StringComparison.Ordinal) && equals > 0) { name = arg.Substring(0, equals); value = arg.Substring(equals + 1); }
            if (!seen.Add(name)) throw new ArgumentException("option given twice: " + name);
            switch (name)
            {
                case "--jobs":
                    value = value ?? NextValue(args, ref i, name);
                    if (!int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out options.Jobs) || options.Jobs < 0)
                        throw new ArgumentException("--jobs needs a whole number >= 0, got '" + value + "'");
                    break;
                case "--filter": options.Filter = value ?? NextValue(args, ref i, name); break;
                case "--junit":
                    options.JUnitPath = value ?? NextValue(args, ref i, name);
                    if (options.JUnitPath.Length == 0) throw new ArgumentException("--junit needs a path");
                    break;
                case "--include-campaigns": Flag(value, name); options.IncludeCampaigns = true; break;
                case "--include-demo-timing": Flag(value, name); options.IncludeDemoTiming = true; break;
                case "--exclude-campaigns": Flag(value, name); break;
                case "--list": Flag(value, name); options.List = true; break;
                case "--help":
                case "-h": Flag(value, name); options.Help = true; break;
                default: throw new ArgumentException("unknown argument '" + arg + "'");
            }
        }
        return options;
    }

    static string NextValue(string[] args, ref int i, string name)
    {
        if (i + 1 >= args.Length || args[i + 1].StartsWith("--", StringComparison.Ordinal)) throw new ArgumentException(name + " needs a value");
        return args[++i];
    }

    static void Flag(string value, string name)
    {
        if (value != null) throw new ArgumentException(name + " takes no value");
    }

    static bool Selected(string name, Options options)
    {
        if (!options.IncludeCampaigns && name.StartsWith("campaign.", StringComparison.Ordinal)) return false;
        if (!options.IncludeDemoTiming && name.StartsWith("demo_timing.", StringComparison.Ordinal)) return false;
        return name.IndexOf(options.Filter, StringComparison.OrdinalIgnoreCase) >= 0;
    }

    static Outcome RunCase(Scenario scenario, RoutingWriter router)
    {
        var buffer = new StringBuilder();
        var outcome = new Outcome();
        router.Current.Value = buffer;
        var clock = Stopwatch.StartNew();
        try { scenario.Run(); outcome.Passed = true; }
        catch (Exception e) { outcome.Error = e; }
        finally
        {
            outcome.Milliseconds = clock.Elapsed.TotalMilliseconds;
            router.Current.Value = null;
            outcome.Output = buffer.ToString();
        }
        return outcome;
    }

    static Exception Unwrap(Exception e)
    {
        while (true)
        {
            if (e is TargetInvocationException && e.InnerException != null) e = e.InnerException;
            else if (e is AggregateException aggregate && aggregate.InnerExceptions.Count == 1) e = aggregate.InnerExceptions[0];
            else return e;
        }
    }

    static IEnumerable<string> TopFrames(Exception e, int count) =>
        (e.StackTrace ?? "").Split('\n').Select(line => line.Trim()).Where(line => line.Length > 0).Take(count);

    static void WriteJUnit(string path, Scenario[] cases, Outcome[] outcomes, double seconds)
    {
        string directory = Path.GetDirectoryName(Path.GetFullPath(path));
        if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
        string Seconds(double value) => value.ToString("F3", CultureInfo.InvariantCulture);
        int failures = outcomes.Count(o => !o.Passed);
        using var writer = XmlWriter.Create(path, new XmlWriterSettings { Indent = true, Encoding = new UTF8Encoding(false) });
        writer.WriteStartDocument();
        writer.WriteStartElement("testsuites");
        writer.WriteAttributeString("tests", cases.Length.ToString(CultureInfo.InvariantCulture));
        writer.WriteAttributeString("failures", failures.ToString(CultureInfo.InvariantCulture));
        writer.WriteAttributeString("time", Seconds(seconds));
        writer.WriteStartElement("testsuite");
        writer.WriteAttributeString("name", "Forest.Scenarios");
        writer.WriteAttributeString("tests", cases.Length.ToString(CultureInfo.InvariantCulture));
        writer.WriteAttributeString("failures", failures.ToString(CultureInfo.InvariantCulture));
        writer.WriteAttributeString("errors", "0");
        writer.WriteAttributeString("skipped", "0");
        writer.WriteAttributeString("time", Seconds(seconds));
        foreach (int i in Enumerable.Range(0, cases.Length).OrderBy(i => cases[i].Name, StringComparer.Ordinal))
        {
            string name = cases[i].Name;
            int dot = name.IndexOf('.');
            writer.WriteStartElement("testcase");
            writer.WriteAttributeString("classname", dot > 0 ? name.Substring(0, dot) : "scenario");
            writer.WriteAttributeString("name", name);
            writer.WriteAttributeString("time", Seconds(outcomes[i].Milliseconds / 1000));
            if (!outcomes[i].Passed)
            {
                var error = Unwrap(outcomes[i].Error);
                writer.WriteStartElement("failure");
                writer.WriteAttributeString("type", error.GetType().FullName);
                writer.WriteAttributeString("message", error.Message);
                writer.WriteString(error.ToString());
                writer.WriteEndElement();
            }
            if (outcomes[i].Output.Length > 0) writer.WriteElementString("system-out", outcomes[i].Output);
            writer.WriteEndElement();
        }
        writer.WriteEndElement();
        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    /// <summary>Routes console writes from a running case into that case's buffer so parallel output never interleaves.</summary>
    sealed class RoutingWriter : TextWriter
    {
        public readonly AsyncLocal<StringBuilder> Current = new AsyncLocal<StringBuilder>();
        readonly TextWriter fallback;
        readonly object gate = new object();
        public RoutingWriter(TextWriter fallback) { this.fallback = fallback; }
        public override Encoding Encoding => fallback.Encoding;
        public override void Write(char value)
        {
            var buffer = Current.Value;
            if (buffer != null) { buffer.Append(value); return; }
            lock (gate) fallback.Write(value);
        }
        public override void Write(string value)
        {
            var buffer = Current.Value;
            if (buffer != null) { buffer.Append(value); return; }
            lock (gate) fallback.Write(value);
        }
    }
}
