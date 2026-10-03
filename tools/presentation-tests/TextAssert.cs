using System.Globalization;
using System.Text.RegularExpressions;
using NUnit.Framework;

namespace IdleCatForest.Presentation.Tests
{
    /// <summary>
    /// Runs every test under a German culture, so any number formatted with the current culture shows
    /// up as a decimal comma and fails the clean-text check.
    /// </summary>
    [SetUpFixture]
    public sealed class GermanCulture
    {
        private CultureInfo culture, uiCulture;

        [OneTimeSetUp]
        public void Enter()
        {
            culture = CultureInfo.CurrentCulture;
            uiCulture = CultureInfo.CurrentUICulture;
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("de-DE");
            CultureInfo.CurrentUICulture = CultureInfo.GetCultureInfo("de-DE");
        }

        [OneTimeTearDown]
        public void Leave()
        {
            CultureInfo.CurrentCulture = culture;
            CultureInfo.CurrentUICulture = uiCulture;
        }
    }

    /// <summary>The rules every piece of player text follows (plan rule 2.9 and GUIDE section 7).</summary>
    public static class TextAssert
    {
        /// <summary>An entity id such as "cat-25", "job-55" or "station_store-18".</summary>
        public static readonly Regex EntityId = new Regex(@"\b[a-z]+(?:_[a-z]+)*-\d+\b");

        /// <summary>A comma between digits that is not a thousands separator ("0,4", "24,0", "-8,-8").</summary>
        public static readonly Regex DecimalComma = new Regex(@"\d,(?:-\d|\d(?!\d{2}(?!\d)))");

        public static bool IsClean(string text, out string problem)
        {
            problem = "";
            if (text == null)
                problem = "null";
            else if (text.Contains("_"))
                problem = "contains an underscore";
            else if (EntityId.IsMatch(text))
                problem = "contains an entity id: " + EntityId.Match(text).Value;
            else if (DecimalComma.IsMatch(text))
                problem = "contains a decimal comma or coordinates";
            else if (text.Contains("NaN") || text.Contains("Infinity") || text.Contains("∞"))
                problem = "contains a non-finite number";
            else if (text.Contains("  "))
                problem = "contains a double space";
            return problem == "";
        }

        /// <summary>Asserts that <paramref name="text"/> is non-empty, readable player text.</summary>
        public static void Clean(string text, string context = "")
        {
            Assert.That(text, Is.Not.Null.And.Not.Empty, "Empty player text " + context);
            Assert.That(IsClean(text, out string problem), Is.True, "\"" + text + "\" " + problem + " " + context);
        }

        /// <summary>Asserts that <paramref name="text"/> is readable if present; empty is allowed.</summary>
        public static void CleanOrEmpty(string text, string context = "")
        {
            Assert.That(text, Is.Not.Null, context);
            if (text.Length > 0)
                Assert.That(IsClean(text, out string problem), Is.True, "\"" + text + "\" " + problem + " " + context);
        }
    }
}
