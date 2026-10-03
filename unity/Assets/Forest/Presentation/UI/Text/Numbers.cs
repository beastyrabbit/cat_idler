using System;
using System.Globalization;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Number formatting for player text. Always the invariant culture, so a German or French Mac never
    /// shows "0,4"; group separators from 1,000 up; a no-break space between a number and its unit.
    /// </summary>
    public static class Numbers
    {
        /// <summary>The culture every number in player text is formatted with.</summary>
        public static readonly CultureInfo Culture = CultureInfo.InvariantCulture;

        /// <summary>Between a number and its unit ("9 tiles", "52 s", "10 %"), so the pair never wraps.</summary>
        public const string UnitSpace = " ";

        /// <summary>The estimate mark used before approximate numbers ("≈ 120").</summary>
        public const string About = "≈";

        /// <summary>
        /// A number with at most <paramref name="decimals"/> decimals, trailing zeros dropped, and group
        /// separators: Num(0.4, 1) is "0.4", Num(2.0, 1) is "2", Num(19995) is "19,995".
        /// </summary>
        public static string Num(double value, int decimals = 0)
        {
            if (double.IsNaN(value) || double.IsInfinity(value))
                return "—";
            decimals = Math.Max(0, Math.Min(decimals, 6));
            double rounded = Math.Round(value, decimals, MidpointRounding.AwayFromZero);
            if (rounded == 0)
                rounded = 0; // never "-0"
            string format = decimals == 0 ? "#,0" : "#,0." + new string('#', decimals);
            return rounded.ToString(format, Culture);
        }

        /// <summary>A whole number with group separators, rounded half away from zero.</summary>
        public static string Whole(double value) => Num(value, 0);

        /// <summary>
        /// A whole number rounded down, for amounts compared against a cost: 4.6 research reads "4",
        /// never "5" that the player then cannot spend.
        /// </summary>
        public static string Floor(double value, int decimals = 0)
        {
            if (double.IsNaN(value) || double.IsInfinity(value))
                return "—";
            decimals = Math.Max(0, Math.Min(decimals, 6));
            double scale = Math.Pow(10, decimals);
            return Num(Math.Floor(value * scale + 1e-9) / scale, decimals);
        }

        /// <summary>
        /// A small amount that may be fractional: whole numbers stay whole, values under 10 keep one
        /// decimal ("0.4", "2.5", "12").
        /// </summary>
        public static string Amount(double value) => Math.Abs(value) < 10 ? Num(value, 1) : Whole(value);

        /// <summary>A signed number: "+3", "−2" (with a real minus sign), "0".</summary>
        public static string Signed(double value, int decimals = 0)
        {
            string text = Num(Math.Abs(value), decimals);
            if (text == "0")
                return "0";
            return (value < 0 ? "−" : "+") + text;
        }

        /// <summary>A fraction as a percentage: Percent(0.1) is "10 %", Percent(0.125, 1) is "12.5 %".</summary>
        public static string Percent(double fraction, int decimals = 0) => Num(fraction * 100, decimals) + UnitSpace + "%";

        /// <summary>A signed percentage from a fraction: SignedPercent(0.1) is "+10 %".</summary>
        public static string SignedPercent(double fraction, int decimals = 0) => Signed(fraction * 100, decimals) + UnitSpace + "%";

        /// <summary>"1 study", "2 studies", "1,200 cats": the count with the right noun.</summary>
        public static string Plural(double n, string one, string many) => Whole(n) + " " + (Math.Abs(n - 1) < 1e-9 ? one : many);

        /// <summary>The noun alone for a count: "study" or "studies".</summary>
        public static string Noun(double n, string one, string many) => Math.Abs(n - 1) < 1e-9 ? one : many;

        /// <summary>"9 tiles", "1 tile".</summary>
        public static string Tiles(int tiles) => Whole(tiles) + UnitSpace + (tiles == 1 ? "tile" : "tiles");

        /// <summary>"3 of 6".</summary>
        public static string Of(double done, double total) => Whole(done) + " of " + Whole(total);

        /// <summary>An ordinal for counted places: 1st, 2nd, 3rd, 4th, 11th, 22nd.</summary>
        public static string Ordinal(int n)
        {
            int lastTwo = Math.Abs(n) % 100;
            string suffix = lastTwo >= 11 && lastTwo <= 13 ? "th" : (Math.Abs(n) % 10) switch { 1 => "st", 2 => "nd", 3 => "rd", _ => "th" };
            return n.ToString(Culture) + suffix;
        }

        /// <summary>Small counts as words in running text ("three hunters"); 13 and more as digits.</summary>
        public static string Words(int n)
        {
            string[] words = { "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve" };
            return n >= 0 && n < words.Length ? words[n] : Whole(n);
        }
    }
}
